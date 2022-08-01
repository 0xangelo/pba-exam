use sp_runtime::{
    traits::{CheckedDiv, CheckedMul},
    ArithmeticError::{self, *},
};

pub trait TryMul: CheckedMul {
    fn try_mul(&self, other: &Self) -> Result<Self, ArithmeticError> {
        self.checked_mul(other).ok_or(Overflow)
    }
}

pub trait TryDiv: CheckedDiv {
    fn try_div(&self, other: &Self) -> Result<Self, ArithmeticError> {
        self.checked_div(other).ok_or(DivisionByZero)
    }
}

impl<T: CheckedMul> TryMul for T {}

impl<T: CheckedDiv> TryDiv for T {}
