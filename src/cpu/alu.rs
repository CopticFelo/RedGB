use crate::error::GBError;

pub fn read_u16(lo: &u8, hi: &u8) -> u16 {
    (*hi as u16) << 8 | *lo as u16
}

pub fn write_u16(lo: &mut u8, hi: &mut u8, value: u16) {
    *hi = (value >> 8) as u8;
    *lo = value as u8;
}

pub fn read_bits(num: u8, index: u8, length: u8) -> u8 {
    let mut out = 0;
    let mut index = index;
    for i in 0..length {
        out += ((num >> index) & 1) * 2_u8.pow(i as u32);
        index += 1;
    }
    out
}

pub fn set_bit(num: u8, index: u8, bit: bool) -> u8 {
    let value = bit as u8;
    return num | (value << index);
}

pub fn write_bits(target: &mut u8, index: u8, length: u8, bits: u8) -> Result<(), GBError> {
    if index + length > 8 {
        return Err(GBError::ByteOverflow { length, index });
    }
    let mask: u8 = ((1 << length) - 1) << index;
    *target = (*target & !mask) | (bits << index);

    Ok(())
}

pub fn rotate_left(mut num: u8, mut carry: bool, through_carry: bool) -> (u8, bool) {
    if through_carry {
        let last = read_bits(num, 7, 1);
        let c = carry;
        num <<= 1;
        carry = last == 1;
        num = set_bit(num, 0, c);
        (num, carry)
    } else {
        let last = read_bits(num, 7, 1);
        num = num.rotate_left(1);
        carry = last == 1;
        (num, carry)
    }
}

pub fn rotate_right(mut num: u8, mut carry: bool, through_carry: bool) -> (u8, bool) {
    if through_carry {
        let first = read_bits(num, 0, 1);
        let c = carry;
        num >>= 1;
        carry = first == 1;
        num = set_bit(num, 7, c);
        (num, carry)
    } else {
        let first = read_bits(num, 0, 1);
        num = num.rotate_right(1);
        carry = first == 1;
        (num, carry)
    }
}

#[test]
fn read_bits_test() {
    assert_eq!(read_bits(0x0E, 6, 1), 0);
}

#[test]
fn rotate_test() {
    let mut num = 0b10000000;
    let mut carry = true;
    (num, carry) = rotate_left(num, carry, true);
    assert_eq!(num, 0b00000001);
    assert!(carry);
    (num, carry) = rotate_right(num, carry, true);
    assert_eq!(num, 0b10000000);
    assert!(carry);
    (num, carry) = rotate_right(num, carry, false);
    assert_eq!(num, 0b01000000);
    assert!(!carry);
}
