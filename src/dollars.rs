use core::{
    fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dollars {
    cents: i64,
}

impl Dollars {
    pub const ZERO: Self = Self { cents: 0 };

    pub fn to_cents(self) -> i64 {
        self.cents
    }

    #[expect(unused)]
    pub fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    pub fn to_f64(self) -> f64 {
        self.cents as f64 / 100.0
    }

    pub fn from_f64(dollars: f64) -> Self {
        Self {
            cents: (dollars * 100.0).round() as i64,
        }
    }
}

impl Add for Dollars {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cents: self.cents + rhs.cents,
        }
    }
}

impl AddAssign for Dollars {
    fn add_assign(&mut self, rhs: Self) {
        self.cents += rhs.cents;
    }
}

impl Sub for Dollars {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            cents: self.cents - rhs.cents,
        }
    }
}

impl SubAssign for Dollars {
    fn sub_assign(&mut self, rhs: Self) {
        self.cents -= rhs.cents;
    }
}

impl Serialize for Dollars {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.cents as f64 / 100.0)
    }
}

impl<'de> Deserialize<'de> for Dollars {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DollarsVisitor;

        impl<'de> de::Visitor<'de> for DollarsVisitor {
            type Value = Dollars;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a floating point number with two decimal places")
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Dollars::from_f64(v as f64))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Dollars::from_f64(v))
            }
        }

        deserializer.deserialize_f64(DollarsVisitor)
    }
}

impl fmt::Display for Dollars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = self.cents.signum();
        let cents = self.cents.abs();

        let digits = usize::max(cents.checked_ilog10().unwrap_or(0) as usize + 1, 3);
        let width = if sign < 0 || f.sign_plus() { 1 } else { 0 } // sign
            + 1 // $
            + digits // digits
            + digits.saturating_sub(3) / 3 // separators
            + 1; // .
        if f.align().is_some_and(|a| a == fmt::Alignment::Right)
            && let Some(w) = f.width()
            && width < w
        {
            write!(f, "{:1$}", "", w - width)?;
        }

        if sign < 0 {
            write!(f, "-")?;
        } else if f.sign_plus() {
            write!(f, "+")?;
        }
        write!(f, "$")?;

        if cents >= 100 {
            let mut dollars = cents / 100;
            let mut digits = dollars.ilog10() + 1;
            let mut is_first_group = true;
            while digits > 0 {
                let mut group_size = digits % 3;
                if group_size == 0 {
                    group_size = 3;
                }
                let base = 10_i64.pow(digits - group_size);
                let segment = dollars / base;
                if is_first_group {
                    write!(f, "{segment}")?;
                    is_first_group = false;
                } else {
                    write!(f, ",{segment:03}")?;
                }
                dollars -= segment * base;
                digits -= group_size;
            }
        } else {
            write!(f, "0")?;
        }

        write!(f, ".{:02}", cents % 100)?;

        if f.align().is_none_or(|a| a == fmt::Alignment::Left)
            && let Some(w) = f.width()
            && width < w
        {
            write!(f, "{:1$}", "", w - width)?;
        }

        Ok(())
    }
}
