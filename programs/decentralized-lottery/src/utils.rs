// src/utils.rs
use anchor_lang::prelude::*;
// ADD this import:
use crate::errors::LotteryError;

pub fn safe_add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(LotteryError::SafeMathError.into())
}

pub fn safe_sub(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b).ok_or(LotteryError::SafeMathError.into())
}

pub fn safe_mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(LotteryError::SafeMathError.into())
}

pub fn safe_div(a: u64, b: u64) -> Result<u64> {
    a.checked_div(b).ok_or(LotteryError::SafeMathError.into())
}