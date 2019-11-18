use std::result::Result as StdResult;

use log::{
    debug,
    error,
};
use rand::prelude::*;
use serenity::{
    framework::standard::{
        Args,
        CommandResult,
        macros::command,
    },
    model::channel::Message,
    prelude::*,
};
use statrs;
use thiserror::Error;

use lazy_static::lazy_static;

use crate::{
    Result,
    util::CtxExt,
};

#[derive(Parser)]
#[grammar = "commands/calc.pest"]
struct Calc;

#[derive(Copy, Clone, Error, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CalcError {
    #[error("pest was unable to parse the input")]
    Pest,

    #[error("invalid number format")]
    NumberFormat,

    #[error("bad argument count")]
    ArgCount,
}

impl Calc {
    fn eval<S: AsRef<str>>(s: S) -> StdResult<f64, CalcError> {
        use pest::{
            Parser,
            prec_climber::PrecClimber,
            iterators::{Pair, Pairs},
        };

        use self::Rule::*;

        lazy_static! {
            static ref CLIMBER: PrecClimber<self::Rule> = {
                use pest::prec_climber::{
                    Operator,
                    Assoc::*,
                };

                PrecClimber::new(vec![
                    Operator::new(add, Left) | Operator::new(sub, Left) | Operator::new(modulo, Left),
                    Operator::new(mul, Left) | Operator::new(div, Left),
                    Operator::new(dice, Left),
                    Operator::new(pow, Right),
                ])
            };
        }

        let result = Calc::parse(calc, s.as_ref()).map_err(|_| CalcError::Pest)?;

        fn eval_single_pair(pair: Pair<self::Rule>) -> StdResult<f64, CalcError> {
            let result = match pair.as_rule() {
                oct | hex | binary => {
                    let base = match pair.as_rule() {
                        hex => 16,
                        oct => 8,
                        binary => 2,
                        _ => unreachable!(),
                    };

                    u64::from_str_radix(&pair.as_str()[2..], base).map_err(|_| CalcError::NumberFormat)? as f64
                },
                float => pair.as_str().parse::<f64>().map_err(|_| CalcError::NumberFormat)?,
                expr | num => eval_expr(pair.into_inner())?,
                unary_expr => {
                    let mut p = pair.into_inner();

                    let op = p.next().ok_or(CalcError::ArgCount)?;
                    let arg = eval_expr(p)?;

                    match op.as_rule() {
                        log => arg.ln(),
                        sqrt => arg.sqrt(),
                        sgn => arg.signum(),

                        sin => arg.sin(),
                        cos => arg.cos(),
                        tan => arg.tan(),
                        asin => arg.asin(),
                        acos => arg.acos(),
                        atan => arg.atan(),

                        sinh => arg.sinh(),
                        cosh => arg.cosh(),
                        tanh => arg.tanh(),
                        asinh => arg.asinh(),
                        acosh => arg.acosh(),
                        atanh => arg.atanh(),

                        exp => arg.exp(),
                        abs => arg.abs(),
                        ceil => arg.ceil(),
                        floor => arg.floor(),
                        round => arg.round(),
                        _ => unreachable!(),
                    }
                },
                binary_expr => {
                    let mut p = pair.into_inner();

                    let op = p.next().ok_or(CalcError::ArgCount)?;

                    let arg1 = eval_single_pair(p.next().ok_or(CalcError::ArgCount)?)?;
                    let arg2 = eval_single_pair(p.next().ok_or(CalcError::ArgCount)?)?;

                    assert!(p.next().is_none());

                    match op.as_rule() {
                        min => arg1.min(arg2),
                        max => arg1.max(arg2),
                        atan2 => arg1.atan2(arg2),
                        _ => unreachable!(),
                    }
                },
                suffix_expr => {
                    let mut p = pair.into_inner();

                    let arg = eval_expr(p.next().ok_or(CalcError::ArgCount)?.into_inner())?;
                    let op = p.next().ok_or(CalcError::ArgCount)?;

                    assert!(p.next().is_none());

                    match op.as_rule() {
                        factorial => statrs::function::gamma::gamma(arg + 1.),
                        _ => unreachable!(),
                    }
                },
                _ => unreachable!(),
            };

            Ok(result)
        }

        fn eval_expr(p: Pairs<self::Rule>) -> StdResult<f64, CalcError> {
            CLIMBER.climb(
                p,
                eval_single_pair,
                |lhs, op, rhs| {
                    let lhs = lhs?;
                    let rhs = rhs?;

                    let result = match op.as_rule() {
                        add => lhs + rhs,
                        sub => lhs - rhs,
                        mul => lhs * rhs,
                        div => lhs / rhs,
                        pow => lhs.powf(rhs),
                        dice => {
                            let dice_count = lhs as usize;
                            let dice_faces = rhs as usize;

                            let mut rng = thread_rng();
                            (0..dice_count).map(|_| rng.gen_range(1, dice_faces + 1)).sum::<usize>() as f64
                        },
                        _ => unreachable!(),
                    };

                    Ok(result)
                }
            )
        }

        eval_expr(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calc_basics() {
        assert_eq!(3., Calc::eval("1 + 2").unwrap());
        assert_eq!(3.0f64.ln(), Calc::eval("log 3").unwrap());
        assert!(6. - Calc::eval("3!").unwrap() < 0.0001);
        assert_eq!(3., Calc::eval("max 3 2").unwrap());
    }

    #[test]
    fn test_binary_unary() {
        assert_eq!(3.0f64.ln(), Calc::eval("max log 3 log 2").unwrap());
    }

    #[test]
    fn test_prefix_suffix() {
        assert!(6. - Calc::eval("abs 3!").unwrap() < 0.0001);
    }
}

#[command]
#[aliases("calc", "calculate")]
pub fn roll(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    match Calc::eval(args.rest()) {
        Ok(result) => {
            debug!("got calc result '{}'", result);
            ctx.send(msg.channel_id, &format!("{}", result), msg.tts)
        },
        Err(e) => {
            error!("error encountered reading calc '{}': {}", args.rest(), e);
            ctx.send(msg.channel_id, "I COULDN'T READ THAT YOU FUCK", msg.tts)
        },
    }
}
