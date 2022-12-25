use crate::helpers::VariableStore;
use acir::{
    circuit::{directives::Directive, Opcode},
    native_types::{Expression, Witness},
};
use acir_field::FieldElement;

// TODO: instead of passing a `new_gates` parameter, we can just return a vector and it is upto the caller to deal with it
// TODO This does not hurt performance at all because we are not reallocating capacity for the new_gates anyways.

// Perform bit decomposition on the provided expression
#[deprecated(note = "use bit_decomposition function instead")]
pub fn split(
    gate: Expression,
    bit_size: u32,
    num_witness: u32,
    new_gates: &mut Vec<Opcode>,
) -> Vec<Witness> {
    let (bits, _) = bit_decomposition(gate, bit_size, num_witness, new_gates);
    bits
}

// Generates opcodes and directives to bit decompose the input `gate`
// Returns the bits and the updated witness counter
// TODO:Ideally, we return the updated witness counter, or we require the input
// TODO to be a VariableStore. We are not doing this because we want migration to
// TODO be less painful
pub fn bit_decomposition(
    gate: Expression,
    bit_size: u32,
    mut num_witness: u32,
    new_gates: &mut Vec<Opcode>,
) -> (Vec<Witness>, u32) {
    let mut variables = VariableStore::new(&mut num_witness);

    // First create a witness for each bit
    let mut bit_vector = Vec::with_capacity(bit_size as usize);
    for _ in 0..bit_size {
        bit_vector.push(variables.new_variable())
    }

    // Next create a directive which computes those bits.
    new_gates.push(Opcode::Directive(Directive::Split {
        a: gate.clone(),
        b: bit_vector.clone(),
        bit_size,
    }));

    // Now apply constraints to the bits such that they are the bit decomposition
    // of the input and each bit is actually a bit
    let mut binary_exprs = Vec::new();
    let mut bit_decomp_constraint = gate;
    let mut two_pow = FieldElement::one();
    let two = FieldElement::from(2_i128);
    for &bit in &bit_vector {
        // Bit constraint to ensure each bit is a zero or one; bit^2 - bit = 0
        let mut expr = Expression::default();
        expr.term_multiplication(FieldElement::one(), bit, bit);
        expr.term_addition(-FieldElement::one(), bit);
        binary_exprs.push(Opcode::Arithmetic(expr));

        // Constraint to ensure that the bits are constrained to be a bit decomposition
        // of the input
        // ie \sum 2^i * x_i = input
        bit_decomp_constraint.term_addition(-two_pow, bit);
        two_pow = two * two_pow;
    }

    new_gates.extend(binary_exprs);
    bit_decomp_constraint.sort(); // TODO: we have an issue open to check if this is needed. Ideally, we remove it.
    new_gates.push(Opcode::Arithmetic(bit_decomp_constraint));

    (bit_vector, variables.finalise())
}

// Range constraint
pub fn range(
    gate: Expression,
    bit_size: u32,
    num_witness: u32,
    new_gates: &mut Vec<Opcode>,
) -> u32 {
    let (_, updated_witness_counter) = bit_decomposition(gate, bit_size, num_witness, new_gates);
    updated_witness_counter
}

pub fn and(
    a: Expression,
    b: Expression,
    result: Witness,
    bit_size: u32,
    num_witness: u32,
    new_gates: &mut Vec<Opcode>,
) -> u32 {
    // Decompose the operands into bits
    //
    let (a_bits, updated_witness_counter) = bit_decomposition(a, bit_size, num_witness, new_gates);
    let (b_bits, updated_witness_counter) =
        bit_decomposition(b, bit_size, updated_witness_counter, new_gates);

    assert_eq!(a_bits.len(), b_bits.len());
    assert_eq!(a_bits.len(), bit_size as usize);

    let mut two_pow = FieldElement::one();
    let two = FieldElement::from(2_i128);

    // Build an expression that Multiplies each bit element-wise
    // This gives the same truth table as the AND operation
    // Additionally, we multiply by a power of 2 to build up the
    // expected output; ie result = \sum 2^i x_i * y_i
    let mut and_expr = Expression::default();
    for (a_bit, b_bit) in a_bits.into_iter().zip(b_bits) {
        and_expr.term_multiplication(two_pow, a_bit, b_bit);
        two_pow = two * two_pow;
    }
    and_expr.term_addition(-FieldElement::one(), result);

    and_expr.sort();
    new_gates.push(Opcode::Arithmetic(and_expr));

    updated_witness_counter
}

pub fn xor(
    a: Expression,
    b: Expression,
    result: Witness,
    bit_size: u32,
    num_witness: u32,
    new_gates: &mut Vec<Opcode>,
) -> u32 {
    // Decompose the operands into bits
    //
    let (a_bits, updated_witness_counter) = bit_decomposition(a, bit_size, num_witness, new_gates);
    let (b_bits, updated_witness_counter) =
        bit_decomposition(b, bit_size, updated_witness_counter, new_gates);

    assert_eq!(a_bits.len(), b_bits.len());
    assert_eq!(a_bits.len(), bit_size as usize);

    let mut two_pow = FieldElement::one();
    let two = FieldElement::from(2_i128);

    // Build an xor expression
    // TODO: check this is the correct arithmetisation
    let mut xor_expr = Expression::default();
    for (a_bit, b_bit) in a_bits.into_iter().zip(b_bits) {
        xor_expr.term_addition(two_pow, a_bit);
        xor_expr.term_addition(two_pow, b_bit);
        two_pow = two * two_pow;
        xor_expr.term_multiplication(-two_pow, a_bit, b_bit);
    }
    xor_expr.term_addition(-FieldElement::one(), result);

    xor_expr.sort();
    new_gates.push(Opcode::Arithmetic(xor_expr));

    updated_witness_counter
}