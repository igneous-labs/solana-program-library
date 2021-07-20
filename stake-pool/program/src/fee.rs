//! Struct representing fees to be applied as a proportion of an amount

use {
    crate::error::StakePoolError,
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    std::{
        cmp::{max, Ordering},
        ops,
    },
};

/// Maximum precision: max value for Fee's denominator, must be < sqrt(u64::MAX)
pub const MAX_FEE_PRECISION: u64 = 1_000_000_000;

/// Fee rate as a ratio, minted on `UpdateStakePoolBalance` as a proportion of
/// the rewards
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Fee {
    /// numerator of the fee ratio
    numerator: u64,
    /// denominator of the fee ratio
    denominator: u64,
}

impl Fee {
    /// Create a new Fee struct. Fails and returns None if:
    /// denominator < 1, denominator > MAX_FEE_PRECISION or numerator > denominator
    pub fn try_new(numerator: u64, denominator: u64) -> Result<Self, StakePoolError> {
        let res = Self {
            numerator,
            denominator,
        };
        res.check()?;
        Ok(res)
    }

    /// Creates a new Fee struct that represents 0 fees
    pub fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    /// Checks if this Fee struct is valid.
    /// Should call this immediately upon deserialization since
    /// user inputted data can be arbitrary u64s
    pub fn check(&self) -> Result<(), StakePoolError> {
        if self.denominator == 0 || self.denominator > MAX_FEE_PRECISION {
            return Err(StakePoolError::InvalidFeeDenominator);
        }
        if self.numerator > self.denominator {
            return Err(StakePoolError::FeeTooHigh);
        }
        Ok(())
    }

    /// Applies the Fee's rates to a given amount, `amt`
    /// returning the amount to be subtracted from it as fees
    pub fn apply(&self, amt: u64) -> u64 {
        let amt_expanded = amt as u128;
        let numerator_expanded = self.numerator as u128;
        let denominator_expanded = self.denominator as u128;
        // overflow safety: both amt_expanded and numerator_expanded are u64
        // div safety: denominator != 0
        let fees = amt_expanded * numerator_expanded / denominator_expanded;
        // as safety: numerator / denominator <= 1.  fees <= amt_expanded <= u64::MAX
        fees as u64
    }
}

impl PartialEq for Fee {
    fn eq(&self, other: &Self) -> bool {
        // multiplication overflow safety:
        // numerator <= denominator <= MAX_FEE_PRECISION < sqrt(u64::max)
        let self_num_common_denom = self.numerator * other.denominator;
        let other_num_common_denom = other.numerator * self.denominator;
        self_num_common_denom == other_num_common_denom
    }
}

impl Eq for Fee {}

impl PartialOrd for Fee {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fee {
    fn cmp(&self, other: &Self) -> Ordering {
        // multiplication overflow safety:
        // numerator <= denominator <= MAX_FEE_PRECISION < sqrt(u64::max)
        let self_num_common_denom = self.numerator * other.denominator;
        let other_num_common_denom = other.numerator * self.denominator;
        self_num_common_denom.cmp(&other_num_common_denom)
    }
}

impl ops::Mul for Fee {
    type Output = Self;

    /// In order to maintain denominator < MAX_FEE_PRECISION,
    /// will result in loss of precision if denominator * denominator > MAX_FEE_PRECISION
    fn mul(self, rhs: Self) -> Self::Output {
        // multiplication overflow safety:
        // numerator <= denominator <= MAX_FEE_PRECISION < sqrt(u64::max)
        let mut numerator = self.numerator * rhs.numerator;
        // numerator <= denominator safety: both numerators <= denominators
        // denominator safety: denominator > 0, since both > 0
        let mut denominator = self.denominator * rhs.denominator;
        if denominator > MAX_FEE_PRECISION {
            // divison safety: MAX_FEE_PRECISION > 0
            let divisor = max(2, denominator / MAX_FEE_PRECISION);
            // division safety: divisor > 0
            // Note: results in loss of precison for numerator if not divisible by divisor
            // or numerator -> 0 if numerator < divisor
            numerator = numerator / divisor;
            denominator = denominator / divisor;
        }
        Self {
            numerator,
            denominator,
        }
    }
}
