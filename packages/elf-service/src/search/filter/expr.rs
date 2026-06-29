mod evaluate;
mod field;
mod parse;
mod serialize;

pub(super) use field::FilterField;

use crate::search::filter::value::FilterValue;

#[derive(Clone, Debug)]
pub(super) enum FilterExpr {
	And(Vec<Self>),
	Or(Vec<Self>),
	Not(Box<Self>),
	Eq { field: FilterField, value: FilterValue },
	Neq { field: FilterField, value: FilterValue },
	In { field: FilterField, values: Vec<FilterValue> },
	Contains { field: FilterField, value: String },
	Gt { field: FilterField, value: FilterValue },
	Gte { field: FilterField, value: FilterValue },
	Lt { field: FilterField, value: FilterValue },
	Lte { field: FilterField, value: FilterValue },
}
impl Default for FilterExpr {
	fn default() -> Self {
		Self::Eq { field: FilterField::Type, value: FilterValue::Null }
	}
}
