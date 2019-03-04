use failure::Error;
use nom::{
    self,
    double,
    types::CompleteStr,
};
use rand::prelude::*;
use serenity::{
    framework::standard::Args,
    model::channel::Message,
    prelude::*,
};
use statrs;

use crate::{
    commands::send,
    Result,
};

#[derive(Clone, Debug, PartialEq)]
enum CalcExpr {
    Binary(BinOp, Box<CalcExpr>, Box<CalcExpr>),
    Unary(UnaryOp, Box<CalcExpr>),
    Term(f64),
}

#[derive(Clone, Debug, PartialEq, Fail)]
enum CalcParseError {
    #[fail(display = "couldn't consume entire expression. remaining: '{}'.", remaining)]
    NotReadToEnd {
        remaining: String,
    },
    #[fail(display = "nom error: {}", _0)]
    Nom(String),
}

impl CalcExpr {
    fn parse<S: AsRef<str>>(input: S) -> Result<Box<Self>> {
        parse_expr(CompleteStr(input.as_ref()))
            .map_err(|e| CalcParseError::Nom(format!("{}", e)))
            .and_then(|(s, res)| {
                if s.len() != 0 {
                    Err(CalcParseError::NotReadToEnd {
                        remaining: s.as_ref().to_owned(),
                    })
                } else {
                    Ok(res)
                }
            })
            .map_err(Error::from)
    }

    fn compute(self: Box<Self>) -> f64 {
        use self::CalcExpr::*;
        use self::BinOp::*;
        use self::UnaryOp::*;

        let s = *self;
        match s {
            Binary(bop, e1, e2) => {
                let r1 = e1.compute();
                let r2 = e2.compute();
                match bop {
                    Add => r1 + r2,
                    Sub => r1 - r2,
                    Mul => r1 * r2,
                    Div => r1 / r2,
                    Mod => r1 % r2,
                    Pow => r1.powf(r2),
                    Min => r1.min(r2),
                    Max => r1.max(r2),
                    DiceRoll => {
                        let dice_count = r1 as usize;
                        let dice_faces = r2 as usize;

                        let mut rng = thread_rng();
                        (0..dice_count).map(|_| rng.gen_range(1, dice_faces + 1)).sum::<usize>() as f64
                    }
                }
            },
            Unary(uop, e) => {
                let r = e.compute();

                match uop {
                    Neg => -r,
                    Log => r.ln(),
                    Sqrt => r.sqrt(),
                    Sgn => r.signum(),
                    Sin => r.sin(),
                    Cos => r.cos(),
                    Tan => r.tan(),
                    Factorial => statrs::function::gamma::gamma(r),
                    Exp => r.exp(),
                    Abs => r.abs(),
                    Ceil => r.ceil(),
                    Floor => r.floor(),
                    Round => r.round(),
                }
            },
            Term(v) => v,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Min,
    Max,
    DiceRoll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnaryOp {
    Log,
    Sqrt,
    Sgn,
    Exp,
    Sin,
    Cos,
    Tan,
    Factorial,
    Neg,
    Ceil,
    Floor,
    Abs,
    Round,
}

fn parse_expr(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, up_to_add_sub_mod)
}

fn parse_add_sub_mod(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        tpl: tuple!(up_to_div_mul, ws!(one_of!("+-%")), up_to_div_mul) >>
        ({
            let (expr1, op, expr2) = tpl;
            let op = match op {
                '+' => BinOp::Add,
                '-' => BinOp::Sub,
                '%' => BinOp::Mod,
                _ => unreachable!(),
            };
            Box::new(CalcExpr::Binary(op, expr1, expr2))
        })
    ))
}

fn parse_div_mul(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        tpl: tuple!(up_to_binary_prefix, ws!(one_of!("/*")), up_to_binary_prefix) >>
        ({
            let (expr1, op, expr2) = tpl;
            let op = match op {
                '*' => BinOp::Mul,
                '/' => BinOp::Div,
                '^' => BinOp::Pow,
                _ => unreachable!(),
            };
            Box::new(CalcExpr::Binary(op, expr1, expr2))
        })
    ))
}

fn parse_binary_prefix(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        op: ws!(alt_complete!(
            tag!("min") => { |_| BinOp::Min } |
            tag!("max") => { |_| BinOp::Max }
        )) >>
        expr1: up_to_unary_prefix >>
        expr2: up_to_unary_prefix >>
        (Box::new(CalcExpr::Binary(op, expr1, expr2)))
    ))
}

fn parse_unary_prefix(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        op: ws!(alt_complete!(
            tag!("log") => { |_| UnaryOp::Log }
            | tag!("sqrt") => { |_| UnaryOp::Sqrt }
            | tag!("sin") => { |_| UnaryOp::Sin }
            | tag!("cos") => { |_| UnaryOp::Cos }
            | tag!("tan") => { |_| UnaryOp::Tan }
            | tag!("sgn") => { |_| UnaryOp::Sgn }
            | tag!("exp") => { |_| UnaryOp::Exp }
            | tag!("ceil") => { |_| UnaryOp::Ceil }
            | tag!("floor") => { |_| UnaryOp::Floor }
            | tag!("abs") => { |_| UnaryOp::Abs }
            | tag!("round") => { |_| UnaryOp::Round }
        )) >>
        expr: up_to_dice >>
        (Box::new(CalcExpr::Unary(op, expr)))
    ))
}

fn parse_dice(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        tpl: separated_pair!(up_to_pow, ws!(char!('d')), up_to_pow) >>
        ({
            let (expr1, expr2) = tpl;
            Box::new(CalcExpr::Binary(BinOp::DiceRoll, expr1, expr2))
        })
    ))
}

fn parse_pow(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        tpl: separated_pair!(up_to_neg, ws!(char!('^')), up_to_neg) >>
        ({
            let (expr1, expr2) = tpl;
            Box::new(CalcExpr::Binary(BinOp::Pow, expr1, expr2))
        })
    ))
}

fn parse_neg(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        expr: ws!(preceded!(char!('-'), up_to_suffix)) >>
        (Box::new(CalcExpr::Unary(UnaryOp::Neg, expr)))
    ))
}

fn parse_suffix(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        expr: terminated!(parse_term_or_paren, ws!(tag!("!"))) >>
        (Box::new(CalcExpr::Unary(UnaryOp::Factorial, expr)))
    ))
}

fn parse_term_or_paren(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, alt_complete!(
        delimited!(char!('('), parse_expr, char!(')')) |
        do_parse!(
            dat: double >>
            (Box::new(CalcExpr::Term(dat)))
        )
    ))
}

macro_rules! up_to {
    ($up_to_name:ident, $fn_name:ident, $prev:ident) => (
        fn $up_to_name(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
            alt_complete!(input, $fn_name | $prev)
        }
    )
}

up_to! { up_to_add_sub_mod, parse_add_sub_mod, up_to_div_mul }
up_to! { up_to_div_mul, parse_div_mul, up_to_binary_prefix }
up_to! { up_to_binary_prefix, parse_binary_prefix, up_to_unary_prefix }
up_to! { up_to_unary_prefix, parse_unary_prefix, up_to_dice }
up_to! { up_to_dice, parse_dice, up_to_pow }
up_to! { up_to_pow, parse_pow, up_to_neg }
up_to! { up_to_neg, parse_neg, up_to_suffix }
up_to! { up_to_suffix, parse_suffix, parse_term_or_paren }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_usize() {
        let (s, expr) = parse_expr("123".into()).unwrap();
        assert_eq!(s.0, "");
        assert_eq!(expr, box CalcExpr::Term(123.));
    }

    #[test]
    fn test_parens() {
        let (s, expr) = parse_expr("(123)".into()).unwrap();
        assert_eq!(s.0, "");
        assert_eq!(expr, box CalcExpr::Term(123.));
    }

    #[test]
    fn test_infix() {
        let (s, expr) = parse_expr("1 + 2".into()).unwrap();
        assert_eq!(s.0, "");
        assert_eq!(expr, box CalcExpr::Binary(BinOp::Add, box CalcExpr::Term(1.), box CalcExpr::Term(2.)))
    }

}

pub fn roll(_ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    match CalcExpr::parse(args.rest()) {
        Ok(expr) => send(msg.channel_id, &format!("{}", expr.compute()), msg.tts),
        Err(e) => {
            let parse_err = e.downcast::<CalcParseError>().unwrap();
            if let CalcParseError::NotReadToEnd { remaining } = parse_err {
                error!("parsing {}: failed to consume '{}'", args.rest(), remaining);
                send(msg.channel_id, "I COULDN'T READ THAT YOU FUCK", msg.tts)
            } else {
                Err(parse_err.into())
            }
        },
    }
}
