use core::num;

use acir::{
    native_types::{Witness, WitnessMap},
    FieldElement
};
use base64::write::StrConsumer;
use bb_rs::barretenberg_api::{acir::get_circuit_sizes, common::example_simple_create_and_verify_proof, srs::init_srs};
use noir_rs::{prove::prove_ultra_honk, srs::setup_srs, utils::decode_circuit, verify::verify_ultra_honk};
use tracing::info;
pub mod prove;
pub mod srs;
pub mod verify;
pub mod utils;
pub mod recursion;
use serde_json;

const BYTECODE: &str = "H4sIAAAAAAAA/62QQQqAMAwErfigpEna5OZXLLb/f4KKLZbiTQdCQg7Dsm66mc9x00O717rhG9ico5cgMOfoMxJu4C2pAEsKioqisnslysoaLVkEQ6aMRYxKFc//ZYQr29L10XfhXv4jB52E+OpMAQAA";

fn main() {
    tracing_subscriber::fmt::init();

    // Setup SRS
    setup_srs(String::from(BYTECODE), None, false).unwrap();

    // Honk
    let mut initial_witness = WitnessMap::new();
    // a
    initial_witness.insert(Witness(0), FieldElement::from(5u128));
    // b
    initial_witness.insert(Witness(1), FieldElement::from(6u128));
    // res = a * b
    initial_witness.insert(Witness(2), FieldElement::from(30u128));

    let start = std::time::Instant::now();
    let (proof, vk) = prove_ultra_honk(String::from(BYTECODE), initial_witness, false).unwrap();
    info!("ultra honk proof generation time: {:?}", start.elapsed());

    let verdict = verify_ultra_honk(proof, vk).unwrap();
    info!("honk proof verification verdict: {}", verdict);
}

#[test]
fn test_common_example() {
    assert!(unsafe { 
        // The group size required to run the example from Barretenberg
        let subgroup_size = 524289;
        let srs = srs::netsrs::NetSrs::new(subgroup_size + 1);
        init_srs(&srs.g1_data, srs.num_points, &srs.g2_data);
        example_simple_create_and_verify_proof() 
    });
}


#[test]
fn test_acir_get_circuit_size() {
    let (_, constraint_system_buf) = decode_circuit(String::from(BYTECODE)).unwrap();
    let circuit_sizes = unsafe { 
        get_circuit_sizes(&constraint_system_buf, false) 
    }; 
    assert_eq!(circuit_sizes.total, 22);
    assert_eq!(circuit_sizes.subgroup, 32);
}


#[test]
fn test_ultra_honk_recursive_proving() {
    // Read the JSON manifest of the circuit 
    let recursed_circuit_txt = std::fs::read_to_string("../../circuits/target/recursed.json").unwrap();
    // Parse the JSON manifest into a dictionary
    let recursed_circuit: serde_json::Value = serde_json::from_str(&recursed_circuit_txt).unwrap();
    // Get the bytecode from the dictionary
    let recursed_circuit_bytecode = recursed_circuit["bytecode"].as_str().unwrap();

    setup_srs(String::from(recursed_circuit_bytecode), None, true).unwrap();

    let mut initial_witness = WitnessMap::new();
    // x
    initial_witness.insert(Witness(0), FieldElement::from(5u128));
    // y
    initial_witness.insert(Witness(1), FieldElement::from(25u128));

    let (recursed_proof, recursed_vk) = prove_ultra_honk(String::from(recursed_circuit_bytecode), initial_witness, true).unwrap();

    let (proof_as_fields, vk_as_fields, key_hash) = recursion::generate_recursive_honk_proof_artifacts(recursed_proof, recursed_vk).unwrap();

    //println!("proof: {:?}", proof_as_fields);
    //println!("vk: {:?}", vk_as_fields);
    //println!("key_hash: {:?}", key_hash);
    
    assert_eq!(proof_as_fields.len(), 463);
    assert_eq!(vk_as_fields.len(), 128);
    //assert_eq!(key_hash, "0x25240793a378438025d0dbe8a4e197c93ec663864a5c9b01699199423dab1008");

    // Read the JSON manifest of the circuit 
    let recursive_circuit_txt = std::fs::read_to_string("../../circuits/target/recursive.json").unwrap();
    // Parse the JSON manifest into a dictionary
    let recursive_circuit: serde_json::Value = serde_json::from_str(&recursive_circuit_txt).unwrap();
    // Get the bytecode from the dictionary
    let recursive_circuit_bytecode = recursive_circuit["bytecode"].as_str().unwrap();
    println!("recursive_circuit_bytecode: {:?}", recursive_circuit_bytecode);

    // IMPORTANT: Likely to run into a timeout for the net srs, replace None with a path to a local srs file
    // before running this test
    setup_srs(String::from(recursive_circuit_bytecode), None, true).unwrap();

    let mut initial_witness_recursive = WitnessMap::new();
    let mut index = 0;
    // Verification key
    vk_as_fields.iter().for_each(|v| {
        initial_witness_recursive.insert(Witness(index), FieldElement::try_from_str(v).unwrap());
        index += 1;
    });
    // Proof
    proof_as_fields.iter().for_each(|v| {
        initial_witness_recursive.insert(Witness(index), FieldElement::try_from_str(v).unwrap());
        index += 1;
    });
    // Public inputs
    initial_witness_recursive.insert(Witness(index), FieldElement::from(25u128));
    index += 1;
    // Key hash
    initial_witness_recursive.insert(Witness(index), FieldElement::try_from_str(&key_hash).unwrap());

    let (proof, vk) = prove_ultra_honk(String::from(recursive_circuit_bytecode), initial_witness_recursive, true).unwrap();

    let verdict = verify_ultra_honk(proof, vk).unwrap();
    assert!(verdict);
}