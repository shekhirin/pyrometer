use std::collections::BTreeMap;
use crate::range::Range;
use std::ops::*;
use crate::range::range_ops::*;
use crate::context::ContextVarNode;
use crate::{nodes::VarType, analyzer::AnalyzerLike};
use crate::{Concrete, NodeIdx};
use crate::range::{elem::RangeOp, *};
use solang_parser::pt::Loc;


#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum DynSide {
	Min,
	Max
}

impl ToString for DynSide {
    fn to_string(&self) -> String {
        match self {
            Self::Min => "range_min".to_string(),
            Self::Max => "range_max".to_string(),
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Dynamic {
	pub idx: NodeIdx,
	pub side: DynSide,
	pub loc: Loc,
}

impl Dynamic {
	pub fn new(idx: NodeIdx, side: DynSide, loc: Loc) -> Self {
		Self { idx, side, loc }
	}
}

impl RangeElem<Concrete> for Dynamic {
	fn eval(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		let (idx, range_side) = (self.idx, self.side);
        let cvar = ContextVarNode::from(idx).underlying(analyzer);
        match &cvar.ty {
            VarType::BuiltIn(_, maybe_range) => {
                if let Some(range) = maybe_range {
                    match range_side {
                        DynSide::Min => {
                            range.range_min().clone().eval(analyzer)
                        }
                        DynSide::Max => {
                            range.range_max().clone().eval(analyzer)
                        }
                    }
                } else {
                    Elem::Dynamic(self.clone())
                }
            }
            VarType::Concrete(concrete_node) => {
            	Elem::Concrete(
            		RangeConcrete {
            			val: concrete_node.underlying(analyzer).clone(),
            			loc: cvar.loc.unwrap_or(Loc::Implicit)
            		}
            	)
            },
            _ => Elem::Dynamic(self.clone()),
        }
	}

	fn simplify(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		let (idx, range_side) = (self.idx, self.side);
		let var = ContextVarNode::from(idx);

		if !var.is_symbolic(analyzer) {
	        let cvar = var.underlying(analyzer);
	        match &cvar.ty {
	            VarType::BuiltIn(_, maybe_range) => {
	                if let Some(range) = maybe_range {
	                    match range_side {
	                        DynSide::Min => {
	                            range.range_min().clone().eval(analyzer)
	                        }
	                        DynSide::Max => {
	                            range.range_max().clone().eval(analyzer)
	                        }
	                    }
	                } else {
	                    Elem::Dynamic(self.clone())
	                }
	            }
	            VarType::Concrete(concrete_node) => {
	            	Elem::Concrete(
	            		RangeConcrete {
	            			val: concrete_node.underlying(analyzer).clone(),
	            			loc: cvar.loc.unwrap_or(Loc::Implicit)
	            		}
	            	)
	            },
	            _ => Elem::Dynamic(self.clone()),
	        }
	    } else {
	    	Elem::Dynamic(self.clone())
	    }
	}

    fn range_eq(&self, _other: &Self, _analyzer: &impl AnalyzerLike) -> bool {
    	todo!()
    }

    fn range_ord(&self, _other: &Self) -> Option<std::cmp::Ordering> {
    	todo!()
    }

    fn dependent_on(&self) -> Vec<ContextVarNode> {
    	vec![ContextVarNode::from(self.idx)]
    }

    fn update_deps(&mut self, mapping: &BTreeMap<ContextVarNode, ContextVarNode>) {
    	if let Some(new) = mapping.get(&ContextVarNode::from(self.idx)) {
    		self.idx = NodeIdx::from(new.0);
    	}
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RangeConcrete<T> {
	pub val: T,
	pub loc: Loc,
}

impl RangeElem<Concrete> for RangeConcrete<Concrete> {
	fn eval(&self, _analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		Elem::Concrete(self.clone())
	}

	fn simplify(&self, _analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		Elem::Concrete(self.clone())
	}

    fn range_eq(&self, other: &Self, analyzer: &impl AnalyzerLike) -> bool {
    	match (self.val.into_u256(), other.val.into_u256()) {
    		(Some(self_val), Some(other_val)) => return self_val == other_val,
    		_ => {
    			match (&self.val, &other.val) {
    				(Concrete::DynBytes(s), Concrete::DynBytes(o)) => s == o,
    				(Concrete::String(s), Concrete::String(o)) => s == o,
    				(Concrete::DynBytes(s), Concrete::String(o)) => s == o.as_bytes(),
    				(Concrete::String(s), Concrete::DynBytes(o)) => s.as_bytes() == o,
    				(Concrete::Array(a), Concrete::Array(b)) => {
    					if a.len() == b.len() {
	    					a.iter().zip(b.iter()).all(|(a, b)| {
	    						let a = RangeConcrete {
	    							val: a.clone(),
	    							loc: self.loc
	    						};

	    						let b = RangeConcrete {
	    							val: b.clone(),
	    							loc: other.loc
	    						};
	    						
	    						a.range_eq(&b, analyzer)
	    					})
	    				} else {
	    					false
	    				}
    				}
    				_ => false
    			}
    		}
    	}
    }

    fn range_ord(&self, other: &Self) -> Option<std::cmp::Ordering> {
    	match (self.val.into_u256(), other.val.into_u256()) {
    		(Some(self_val), Some(other_val)) => return Some(self_val.cmp(&other_val)),
    		(Some(_), _) => {
    			match other.val {
    				Concrete::Int(_, _) => {
    					// if we couldnt convert an int to uint, its negative
    					// so self must be > other
    					Some(std::cmp::Ordering::Greater)
    				}
    				_ => None
    			}
    		}
    		(_, Some(_)) => {
    			match self.val {
    				Concrete::Int(_, _) => {
    					// if we couldnt convert an int to uint, its negative
    					// so self must be < other
    					Some(std::cmp::Ordering::Less)
    				}
    				_ => None
    			}
    		}
    		_ => {
    			match (&self.val, &other.val) {
    				// two negatives
    				(Concrete::Int(_, s), Concrete::Int(_, o)) => Some(s.cmp(&o)),
    				_ => None

    			}
    		}
    	}
    }

    fn dependent_on(&self) -> Vec<ContextVarNode> {
    	vec![]
    }
    fn update_deps(&mut self, _mapping: &BTreeMap<ContextVarNode, ContextVarNode>) {}
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RangeExpr<T> {
	pub lhs: Box<Elem<T>>,
	pub op: RangeOp,
	pub rhs: Box<Elem<T>>,
}

impl<T> RangeExpr<T> {
	pub fn new(lhs: Elem<T>, op: RangeOp, rhs: Elem<T>) -> RangeExpr<T> {
		RangeExpr {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
        }
	}
}

impl RangeElem<Concrete> for RangeExpr<Concrete> {
	fn eval(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		self.exec_op(analyzer)
	}

	fn simplify(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		self.simplify_exec_op(analyzer)
	}

    fn range_eq(&self, _other: &Self, _analyzer: &impl AnalyzerLike) -> bool {
    	todo!()
    }

    fn range_ord(&self, _other: &Self) -> Option<std::cmp::Ordering> {
    	todo!()
    }

    fn dependent_on(&self) -> Vec<ContextVarNode> {
    	let mut deps = self.lhs.dependent_on();
    	deps.extend(self.rhs.dependent_on());
    	deps
    }

    fn update_deps(&mut self, mapping: &BTreeMap<ContextVarNode, ContextVarNode>) {
    	self.lhs.update_deps(mapping);
    	self.rhs.update_deps(mapping);
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Elem<T> {
	Dynamic(Dynamic),
	Concrete(RangeConcrete<T>),
	Expr(RangeExpr<T>),
	Null,
}

impl<T> Elem<T> {
	pub fn cast(self, other: Self) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Cast,
            rhs: Box::new(other),
        };
        Elem::Expr(expr)
    }

	pub fn min(self, other: Self) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Min,
            rhs: Box::new(other),
        };
        Elem::Expr(expr)
    }

    pub fn max(self, other: Self) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Max,
            rhs: Box::new(other),
        };
        Elem::Expr(expr)
    }

    pub fn eq(self, other: Self) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Eq,
            rhs: Box::new(other),
        };
        Elem::Expr(expr)
    }

    pub fn neq(self, other: Self) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Neq,
            rhs: Box::new(other),
        };
        Elem::Expr(expr)
    }
}

impl<T> From<Dynamic> for Elem<T> {
	fn from(dy: Dynamic) -> Self {
		Elem::Dynamic(dy)
	}
}

impl<T> From<RangeConcrete<T>> for Elem<T> {
	fn from(c: RangeConcrete<T>) -> Self {
		Elem::Concrete(c)
	}
}


impl RangeElem<Concrete> for Elem<Concrete> {
	fn eval(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		use Elem::*;
		match self {
			Dynamic(dy) => dy.eval(analyzer),
			Concrete(inner) => inner.eval(analyzer),
			Expr(expr) => expr.eval(analyzer),
			Null => Elem::Null,
		}
	}

	fn simplify(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		use Elem::*;
		match self {
			Dynamic(dy) => dy.simplify(analyzer),
			Concrete(inner) => inner.simplify(analyzer),
			Expr(expr) => expr.simplify(analyzer),
			Null => Elem::Null,
		}
	}

    fn range_eq(&self, other: &Self, analyzer: &impl AnalyzerLike) -> bool {
    	let lhs = self.eval(analyzer);
    	let rhs = other.eval(analyzer);
    	match (lhs, rhs) {
    		(Self::Concrete(a), Self::Concrete(b)) => {
    			a.range_eq(&b, analyzer)
    		}
    		_ => false
    	}
    }

    fn range_ord(&self, other: &Self) -> Option<std::cmp::Ordering> {
    	match (self, other) {
    		(Self::Concrete(a), Self::Concrete(b)) => {
    			a.range_ord(b)
    		}
    		_ => None,
    	}
    }

    fn dependent_on(&self) -> Vec<ContextVarNode> {
    	match self {
    		Self::Dynamic(d) => d.dependent_on(),
    		Self::Concrete(_) => vec![],
    		Self::Expr(expr) => expr.dependent_on(),
    		Self::Null => vec![]
    	}
    }
	fn update_deps(&mut self, mapping: &BTreeMap<ContextVarNode, ContextVarNode>) {
		match self {
    		Self::Dynamic(d) => d.update_deps(mapping),
    		Self::Concrete(_) => {},
    		Self::Expr(expr) => expr.update_deps(mapping),
    		Self::Null => {},
    	}
	}
}

impl Add for Elem<Concrete> {
    type Output = Self;

    fn add(self, other: Elem<Concrete>) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Add,
            rhs: Box::new(other),
        };
        Self::Expr(expr)
    }
}

impl Sub for Elem<Concrete> {
    type Output = Self;

    fn sub(self, other: Elem<Concrete>) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Sub,
            rhs: Box::new(other),
        };
        Self::Expr(expr)
    }
}

impl Mul for Elem<Concrete> {
    type Output = Self;

    fn mul(self, other: Elem<Concrete>) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Mul,
            rhs: Box::new(other),
        };
        Self::Expr(expr)
    }
}

impl Div for Elem<Concrete> {
    type Output = Self;

    fn div(self, other: Elem<Concrete>) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Div,
            rhs: Box::new(other),
        };
        Self::Expr(expr)
    }
}

impl Shl for Elem<Concrete> {
    type Output = Self;

    fn shl(self, other: Elem<Concrete>) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Shl,
            rhs: Box::new(other),
        };
        Self::Expr(expr)
    }
}

impl Shr for Elem<Concrete> {
    type Output = Self;

    fn shr(self, other: Elem<Concrete>) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Shr,
            rhs: Box::new(other),
        };
        Self::Expr(expr)
    }
}

impl Rem for Elem<Concrete> {
    type Output = Self;

    fn rem(self, other: Elem<Concrete>) -> Self {
        let expr = RangeExpr {
            lhs: Box::new(self),
            op: RangeOp::Mod,
            rhs: Box::new(other),
        };
        Self::Expr(expr)
    }
}



pub trait ExecOp<T> {
	fn exec_op(&self, analyzer: &impl AnalyzerLike) -> Elem<T>;
	fn simplify_exec_op(&self, analyzer: &impl AnalyzerLike) -> Elem<T>;
}

impl ExecOp<Concrete> for RangeExpr<Concrete> {
	fn exec_op(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		let lhs = self.lhs.eval(analyzer);
		let rhs = self.rhs.eval(analyzer);
		match self.op {
			RangeOp::Add => {
				lhs.range_add(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Sub => {
				lhs.range_sub(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Mul => {
				lhs.range_mul(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Div => {
				lhs.range_div(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Mod => {
				lhs.range_mod(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Min => {
				lhs.range_min(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Max => {
				lhs.range_max(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Gt => {
				lhs.range_gt(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Lt => {
				lhs.range_lt(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Gte => {
				lhs.range_gte(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Lte => {
				lhs.range_lte(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Eq => {
				lhs.range_ord_eq(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Neq => {
				lhs.range_neq(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Shl => {
				lhs.range_shl(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Shr => {
				lhs.range_shr(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::And => {
				lhs.range_and(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Not => {
				assert!(matches!(rhs, Elem::Null));
				lhs.range_not().unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Cast => {
				lhs.range_cast(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			_ => todo!()
		}
	}

	fn simplify_exec_op(&self, analyzer: &impl AnalyzerLike) -> Elem<Concrete> {
		let lhs = self.lhs.simplify(analyzer);
		let rhs = self.rhs.simplify(analyzer);
		match self.op {
			RangeOp::Add => {
				lhs.range_add(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Sub => {
				lhs.range_sub(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Mul => {
				lhs.range_mul(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Div => {
				lhs.range_div(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Mod => {
				lhs.range_mod(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Min => {
				lhs.range_min(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Max => {
				lhs.range_max(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Gt => {
				lhs.range_gt(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Lt => {
				lhs.range_lt(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Gte => {
				lhs.range_gte(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Lte => {
				lhs.range_lte(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Eq => {
				lhs.range_ord_eq(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Neq => {
				lhs.range_neq(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Shl => {
				lhs.range_shl(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Shr => {
				lhs.range_shr(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::And => {
				lhs.range_and(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Not => {
				assert!(matches!(rhs, Elem::Null));
				lhs.range_not().unwrap_or(Elem::Expr(self.clone()))
			}
			RangeOp::Cast => {
				lhs.range_cast(&rhs).unwrap_or(Elem::Expr(self.clone()))
			}
			_ => todo!()
		}
	}
}