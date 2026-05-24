use crate::store::Record;

// ── Tokens ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Select, Where, OrderBy, Limit, And, Or, Not, Commands, Workflows,
    Ident(String), Str(String), Num(i64), Float(f64),
    Gt, Lt, Gte, Lte, Eq, Neq, Star, Eof,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut pos = 0;
    let b = input.as_bytes();
    let mut out = vec![];
    while pos < b.len() {
        while pos < b.len() && b[pos].is_ascii_whitespace() { pos += 1; }
        if pos >= b.len() { break; }
        let tok = match b[pos] as char {
            '>' => { pos += 1; if b.get(pos)==Some(&b'=') { pos+=1; Token::Gte } else { Token::Gt } }
            '<' => { pos += 1; if b.get(pos)==Some(&b'=') { pos+=1; Token::Lte } else { Token::Lt } }
            '=' => { pos += 1; Token::Eq }
            '!' => { pos += 1; if b.get(pos)==Some(&b'=') { pos+=1; Token::Neq } else { continue } }
            '*' => { pos += 1; Token::Star }
            '\''|'"' => {
                let q = b[pos]; pos += 1; let start = pos;
                while pos < b.len() && b[pos] != q { pos += 1; }
                let s = input[start..pos].to_string();
                if pos < b.len() { pos += 1; }
                Token::Str(s)
            }
            c if c.is_ascii_digit() || c == '-' => {
                let start = pos;
                if b[pos]==b'-' { pos+=1; }
                while pos < b.len() && (b[pos].is_ascii_digit() || b[pos]==b'.') { pos+=1; }
                let s = &input[start..pos];
                if s.contains('.') { Token::Float(s.parse().unwrap_or(0.0)) } else { Token::Num(s.parse().unwrap_or(0)) }
            }
            c if c.is_alphabetic() || c=='_' => {
                let start = pos;
                while pos < b.len() && (b[pos].is_ascii_alphanumeric() || b[pos]==b'_') { pos+=1; }
                let word = &input[start..pos];
                if word.eq_ignore_ascii_case("ORDER") {
                    let save = pos;
                    while pos < b.len() && b[pos].is_ascii_whitespace() { pos+=1; }
                    let s2 = pos;
                    while pos < b.len() && b[pos].is_ascii_alphabetic() { pos+=1; }
                    if input[s2..pos].eq_ignore_ascii_case("BY") { Token::OrderBy }
                    else { pos = save; Token::Ident(word.to_string()) }
                } else {
                    match word.to_uppercase().as_str() {
                        _ => match word.to_uppercase().as_str() {
                            "SELECT"    => Token::Select,
                            "WHERE"     => Token::Where,
                            "LIMIT"     => Token::Limit,
                            "AND"       => Token::And,
                            "OR"        => Token::Or,
                            "NOT"       => Token::Not,
                            "COMMANDS"  => Token::Commands,
                            "WORKFLOWS" => Token::Workflows,
                            _           => Token::Ident(word.to_string()),
                        }
                    }
                }
            }
            _ => { pos += 1; continue }
        };
        out.push(tok);
    }
    out.push(Token::Eof);
    out
}

// ── AST ───────────────────────────────────────────────────────────────────────

pub enum Source { Commands, Workflows }
enum Value  { Int(i64), Float(f64), Str(String) }
enum Op     { Gt, Lt, Gte, Lte, Eq, Neq }
struct Cond { field: String, op: Op, value: Value }
enum Filter {
    Cond(Cond),
    And(Box<Filter>, Box<Filter>),
    Or(Box<Filter>,  Box<Filter>),
    Not(Box<Filter>),
}
pub struct Query { pub source: Source, filter: Option<Filter>, order_by: Option<String>, limit: Option<usize> }

// ── Parser ────────────────────────────────────────────────────────────────────

struct P { tokens: Vec<Token>, pos: usize }
impl P {
    fn peek(&self) -> &Token { self.tokens.get(self.pos).unwrap_or(&Token::Eof) }
    fn next(&mut self) -> Token { let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof); self.pos += 1; t }

    fn query(&mut self) -> Result<Query, String> {
        if !matches!(self.next(), Token::Select) { return Err("Expected SELECT".into()); }
        if matches!(self.peek(), Token::Star|Token::Ident(_)) { self.next(); }
        let source = match self.next() {
            Token::Commands  => Source::Commands,
            Token::Workflows => Source::Workflows,
            t => return Err(format!("Expected COMMANDS or WORKFLOWS, got {:?}", t)),
        };
        let mut q = Query { source, filter: None, order_by: None, limit: None };
        loop { match self.peek() {
            Token::Where   => { self.next(); q.filter   = Some(self.filter()?); }
            Token::OrderBy => { self.next(); q.order_by = Some(match self.next() { Token::Ident(s) => s, _ => return Err("Expected field".into()) }); }
            Token::Limit   => { self.next(); q.limit    = Some(match self.next() { Token::Num(n) => n as usize, _ => return Err("Expected number".into()) }); }
            Token::Eof     => break,
            _              => { self.next(); }
        }}
        Ok(q)
    }

    fn filter(&mut self) -> Result<Filter, String> {
        let left = self.atom()?;
        match self.peek() {
            Token::And => { self.next(); Ok(Filter::And(Box::new(left), Box::new(self.filter()?))) }
            Token::Or  => { self.next(); Ok(Filter::Or(Box::new(left),  Box::new(self.filter()?))) }
            _          => Ok(left),
        }
    }

    fn atom(&mut self) -> Result<Filter, String> {
        if matches!(self.peek(), Token::Not) { self.next(); return Ok(Filter::Not(Box::new(self.atom()?))); }
        let field = match self.next() { Token::Ident(s) => s, t => return Err(format!("Expected field, got {:?}", t)) };
        let op = match self.next() {
            Token::Gt=>Op::Gt, Token::Lt=>Op::Lt, Token::Gte=>Op::Gte,
            Token::Lte=>Op::Lte, Token::Eq=>Op::Eq, Token::Neq=>Op::Neq,
            t => return Err(format!("Expected operator, got {:?}", t)),
        };
        let value = match self.next() {
            Token::Num(n)   => Value::Int(n),
            Token::Float(f) => Value::Float(f),
            Token::Str(s)|Token::Ident(s) => Value::Str(s),
            t => return Err(format!("Expected value, got {:?}", t)),
        };
        Ok(Filter::Cond(Cond { field, op, value }))
    }
}

pub fn parse(input: &str) -> Result<Query, String> {
    P { tokens: tokenize(input), pos: 0 }.query()
}

// ── Executor ──────────────────────────────────────────────────────────────────

pub struct Row { pub command: String, pub frequency: i32, pub score: i32 }

pub fn execute(q: &Query, records: &[&Record]) -> Vec<Row> {
    let mut rows: Vec<Row> = records.iter()
        .map(|r| Row { command: r.command.clone(), frequency: r.frequency, score: r.score })
        .filter(|row| q.filter.as_ref().map_or(true, |f| eval(f, row)))
        .collect();
    match q.order_by.as_deref() {
        Some("frequency") => rows.sort_by_key(|r| std::cmp::Reverse(r.frequency)),
        Some("length")    => rows.sort_by_key(|r| std::cmp::Reverse(r.command.len() as i32)),
        _                 => rows.sort_by_key(|r| std::cmp::Reverse(r.score)),
    }
    if let Some(n) = q.limit { rows.truncate(n); }
    rows
}

fn eval(f: &Filter, row: &Row) -> bool {
    match f {
        Filter::Cond(c)   => eval_cond(c, row),
        Filter::And(a, b) => eval(a, row) && eval(b, row),
        Filter::Or(a, b)  => eval(a, row) || eval(b, row),
        Filter::Not(inner)=> !eval(inner, row),
    }
}

fn eval_cond(c: &Cond, row: &Row) -> bool {
    match c.field.to_lowercase().as_str() {
        "frequency"           => cmp(row.frequency as f64, &c.op, fnum(&c.value)),
        "score"               => cmp(row.score as f64,     &c.op, fnum(&c.value)),
        "length"|"char_len"   => cmp(row.command.len() as f64, &c.op, fnum(&c.value)),
        "command"|"cmd"       => match &c.op {
            Op::Eq  => row.command == fstr(&c.value),
            Op::Neq => row.command != fstr(&c.value),
            _       => row.command.contains(&fstr(&c.value)),
        },
        _ => true,
    }
}

fn fnum(v: &Value) -> f64 { match v { Value::Int(n)=>*n as f64, Value::Float(f)=>*f, Value::Str(s)=>s.parse().unwrap_or(0.0) } }
fn fstr(v: &Value) -> String { match v { Value::Str(s)=>s.clone(), Value::Int(n)=>n.to_string(), Value::Float(f)=>f.to_string() } }
fn cmp(lhs: f64, op: &Op, rhs: f64) -> bool {
    match op { Op::Gt=>lhs>rhs, Op::Lt=>lhs<rhs, Op::Gte=>lhs>=rhs, Op::Lte=>lhs<=rhs,
               Op::Eq=>(lhs-rhs).abs()<f64::EPSILON, Op::Neq=>(lhs-rhs).abs()>=f64::EPSILON }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn basic()    { let q = parse("SELECT * COMMANDS WHERE frequency > 5 LIMIT 10").unwrap(); assert!(matches!(q.source, Source::Commands)); assert_eq!(q.limit, Some(10)); }
    #[test] fn workflows(){ let q = parse("SELECT * WORKFLOWS").unwrap(); assert!(matches!(q.source, Source::Workflows)); assert!(q.filter.is_none()); }
    #[test] fn and_filter(){ let q = parse("SELECT * COMMANDS WHERE frequency > 3 AND score > 10").unwrap(); assert!(matches!(q.filter, Some(Filter::And(_, _)))); }
    #[test] fn order()    { let q = parse("SELECT * COMMANDS ORDER BY frequency LIMIT 5").unwrap(); assert_eq!(q.order_by.as_deref(), Some("frequency")); }
}
