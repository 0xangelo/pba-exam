use sp_runtime::{
    traits::{CheckedDiv, CheckedMul, CheckedSub, CheckedAdd},
    ArithmeticError::{self, *},
};

pub trait TryAdd: CheckedAdd {
    fn try_add(&self, other: &Self) -> Result<Self, ArithmeticError> {
        self.checked_add(other).ok_or(Overflow)
    }
}

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

pub trait TrySub: CheckedSub {
    fn try_sub(&self, other: &Self) -> Result<Self, ArithmeticError> {
        self.checked_sub(other).ok_or(Underflow)
    }
}

impl<T: CheckedAdd> TryAdd for T {}

impl<T: CheckedMul> TryMul for T {}

impl<T: CheckedDiv> TryDiv for T {}

impl<T: CheckedSub> TrySub for T {}
