//! CSS resolution values.

use super::angle::impl_try_from_angle;
use super::calc::Calc;
use super::length::serialize_dimension;
use super::number::{CSSNumber, NumericRange};
use crate::compat::Feature;
use crate::error::{ParserError, PrinterError};
use crate::printer::Printer;
use crate::traits::private::AddInternal;
use crate::traits::{Map, Op, Parse, ParseNumeric, Sign, ToCss};
#[cfg(feature = "visitor")]
use crate::visitor::Visit;
use cssparser::*;

/// A CSS [`<resolution>`](https://www.w3.org/TR/css-values-4/#resolution) value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "visitor", visit(visit_resolution, RESOLUTIONS))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum Resolution {
  /// A resolution in dots per inch.
  Dpi(CSSNumber),
  /// A resolution in dots per centimeter.
  Dpcm(CSSNumber),
  /// A resolution in dots per px.
  Dppx(CSSNumber),
}

impl Resolution {
  pub(crate) fn to_dppx(&self) -> CSSNumber {
    match *self {
      Self::Dpi(value) => value / 96.0,
      Self::Dpcm(value) => value * 2.54 / 96.0,
      Self::Dppx(value) => value,
    }
  }
}

impl<'i> Parse<'i> for Resolution {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    Self::parse_with_range(input, NumericRange::All)
  }
}

impl<'i> ParseNumeric<'i> for Resolution {
  fn parse_with_range<'t>(
    input: &mut Parser<'i, 't>,
    range: NumericRange,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    match input.try_parse(Calc::<Self>::parse) {
      Ok(Calc::Value(value)) => return Ok(value.map(|value| range.clamp(value))),
      Ok(_) => return Err(input.new_custom_error(ParserError::InvalidValue)),
      Err(_) => {}
    }

    let location = input.current_source_location();
    match *input.next()? {
      Token::Dimension { value, ref unit, .. } => {
        let value = range.check(value, location)?;
        match_ignore_ascii_case! { unit,
          "dpi" => Ok(Resolution::Dpi(value)),
          "dpcm" => Ok(Resolution::Dpcm(value)),
          "dppx" | "x" => Ok(Resolution::Dppx(value)),
          _ => Err(location.new_unexpected_token_error(Token::Ident(unit.clone())))
        }
      }
      ref t => Err(location.new_unexpected_token_error(t.clone())),
    }
  }
}

impl<'i> TryFrom<&Token<'i>> for Resolution {
  type Error = ();

  fn try_from(token: &Token) -> Result<Self, Self::Error> {
    match token {
      Token::Dimension { value, ref unit, .. } => match_ignore_ascii_case! { unit,
        "dpi" => Ok(Resolution::Dpi(*value)),
        "dpcm" => Ok(Resolution::Dpcm(*value)),
        "dppx" | "x" => Ok(Resolution::Dppx(*value)),
        _ => Err(()),
      },
      _ => Err(()),
    }
  }
}

impl ToCss for Resolution {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    let (value, unit) = match self {
      Resolution::Dpi(dpi) => (*dpi, "dpi"),
      Resolution::Dpcm(dpcm) => (*dpcm, "dpcm"),
      Resolution::Dppx(dppx) => {
        if dest.targets.current.is_compatible(Feature::XResolutionUnit) {
          (*dppx, "x")
        } else {
          (*dppx, "dppx")
        }
      }
    };

    serialize_dimension(value, unit, dest)
  }
}

impl std::ops::Add<CSSNumber> for Resolution {
  type Output = Self;

  fn add(self, other: CSSNumber) -> Resolution {
    match self {
      Resolution::Dpi(dpi) => Resolution::Dpi(dpi + other),
      Resolution::Dpcm(dpcm) => Resolution::Dpcm(dpcm + other),
      Resolution::Dppx(dppx) => Resolution::Dppx(dppx + other),
    }
  }
}

impl From<Resolution> for Calc<Resolution> {
  fn from(value: Resolution) -> Self {
    Calc::Value(Box::new(value))
  }
}

impl TryFrom<Calc<Resolution>> for Resolution {
  type Error = ();

  fn try_from(calc: Calc<Resolution>) -> Result<Self, Self::Error> {
    match calc {
      Calc::Value(value) => Ok(*value),
      _ => Err(()),
    }
  }
}

impl std::ops::Mul<f32> for Resolution {
  type Output = Self;

  fn mul(self, factor: f32) -> Self {
    self.map(|value| value * factor)
  }
}

impl AddInternal for Resolution {
  fn add(self, other: Self) -> Self {
    self.op(&other, |left, right| left + right)
  }
}

impl PartialOrd for Resolution {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    self.op_to(other, |left, right| left.partial_cmp(&right))
  }
}

impl Op for Resolution {
  fn op<F: FnOnce(f32, f32) -> f32>(&self, rhs: &Self, op: F) -> Self {
    match (self, rhs) {
      (Self::Dpi(a), Self::Dpi(b)) => Self::Dpi(op(*a, *b)),
      (Self::Dpcm(a), Self::Dpcm(b)) => Self::Dpcm(op(*a, *b)),
      _ => Self::Dppx(op(self.to_dppx(), rhs.to_dppx())),
    }
  }

  fn op_to<T, F: FnOnce(f32, f32) -> T>(&self, rhs: &Self, op: F) -> T {
    match (self, rhs) {
      (Self::Dpi(a), Self::Dpi(b)) | (Self::Dpcm(a), Self::Dpcm(b)) => op(*a, *b),
      _ => op(self.to_dppx(), rhs.to_dppx()),
    }
  }
}

impl Map for Resolution {
  fn map<F: FnOnce(f32) -> f32>(&self, op: F) -> Self {
    match *self {
      Self::Dpi(value) => Self::Dpi(op(value)),
      Self::Dpcm(value) => Self::Dpcm(op(value)),
      Self::Dppx(value) => Self::Dppx(op(value)),
    }
  }
}

impl Sign for Resolution {
  fn sign(&self) -> f32 {
    match *self {
      Self::Dpi(value) | Self::Dpcm(value) | Self::Dppx(value) => value.sign(),
    }
  }
}

impl_try_from_angle!(Resolution);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_calc_with_non_resolution_type() {
    for css in ["calc(2)", "calc(1x + 1px)"] {
      let mut input = ParserInput::new(css);
      assert!(Parser::new(&mut input).parse_entirely(Resolution::parse).is_err());
    }
  }
}
