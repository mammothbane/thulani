use failure::err_msg;
use nom::{
    self,
    digit,
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

impl CalcExpr {
    fn parse<S: AsRef<str>>(input: S) -> Result<Box<Self>> {
        parse_expr(CompleteStr(input.as_ref()))
            .map(|(_, res)| res)
            .map_err(|e| err_msg(format!("couldn't parse: {}", e)))
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
}

fn parse_expr(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, alt_complete!(
        parse_infix |
        parse_dice |
        parse_binary_prefix |
        parse_suffix |
        parse_prefix |
        parse_term_or_paren
    ))
}

fn parse_term_or_paren(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, alt_complete!(
        delimited!(char!('('), parse_expr, char!(')')) |
        do_parse!(
            dat: flat_map!(digit, parse_to!(f64)) >>
            (Box::new(CalcExpr::Term(dat)))
        )
    ))
}

fn parse_dice(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        tpl: separated_pair!(parse_term_or_paren, ws!(char!('d')), parse_term_or_paren) >>
        ({
            let (expr1, expr2) = tpl;
            Box::new(CalcExpr::Binary(BinOp::DiceRoll, expr1, expr2))
        })
    ))
}

fn parse_infix(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        tpl: tuple!(parse_term_or_paren, ws!(one_of!("+-*/%^")), parse_term_or_paren) >>
        ({
            let (expr1, op, expr2) = tpl;
            let op = match op {
                '+' => BinOp::Add,
                '-' => BinOp::Sub,
                '*' => BinOp::Mul,
                '/' => BinOp::Div,
                '%' => BinOp::Mod,
                '^' => BinOp::Pow,
                _ => unreachable!(),
            };
            Box::new(CalcExpr::Binary(op, expr1, expr2))
        })
    ))
}

fn parse_prefix(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        op: ws!(alt_complete!(
            tag!("log") => { |_| UnaryOp::Log }
            | tag!("sqrt") => { |_| UnaryOp::Sqrt }
            | tag!("sin") => { |_| UnaryOp::Sin }
            | tag!("cos") => { |_| UnaryOp::Cos }
            | tag!("tan") => { |_| UnaryOp::Tan }
            | tag!("sgn") => { |_| UnaryOp::Sgn }
            | tag!("exp") => { |_| UnaryOp::Exp }
            | char!('-') => { |_| UnaryOp::Neg }
        )) >>
        expr: parse_term_or_paren >>
        (Box::new(CalcExpr::Unary(op, expr)))
    ))
}

fn parse_binary_prefix(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        op: ws!(alt_complete!(
            tag!("min") => { |_| BinOp::Min } |
            tag!("max") => { |_| BinOp::Max }
        )) >>
        expr1: parse_term_or_paren >>
        expr2: parse_term_or_paren >>
        (Box::new(CalcExpr::Binary(op, expr1, expr2)))
    ))
}

fn parse_suffix(input: CompleteStr) -> nom::IResult<CompleteStr, Box<CalcExpr>> {
    ws!(input, do_parse!(
        expr: terminated!(parse_term_or_paren, ws!(tag!("!"))) >>
        (Box::new(CalcExpr::Unary(UnaryOp::Factorial, expr)))
    ))
}

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
    let expr = CalcExpr::parse(args.rest())?;
    send(msg.channel_id, &format!("{}", expr.compute()), msg.tts)
}
