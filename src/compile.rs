use std::fmt::{self, Display, Formatter};
use std::io::Write;

use wasmparser::{FuncType, FunctionBody, Operator, BlockType};

pub struct FunctionDef<'a> {
    pub func_type: FuncType,
    pub body: FunctionBody<'a>,
}

pub struct Module<'a> {
    pub entry: usize,
    pub functions: Vec<FunctionDef<'a>>,
}

impl<'a> Module<'a> {
    pub fn compile(&self, out: &mut Vec<u8>) {
        let mut labeler = Labeler::new();
        writeln!(out, "LD SP,0xFFFB").unwrap();
        writeln!(out, "CALL func_{}", self.entry).unwrap();
        writeln!(out, "HALT").unwrap();
        for (index, func) in self.functions.iter().enumerate() {
            writeln!(out, "func_{}:", index).unwrap();
            self.compile_function(out, &mut labeler, func);
        }
    }

    fn compile_function(&self, out: &mut Vec<u8>, labeler: &mut Labeler, def: &FunctionDef) {
        assert!(def.func_type.results().len() <= 1);
        let params = def.func_type.params();
        let has_result = def.func_type.results().len() == 1;
        let loop_label = labeler.next();
        let operators = def.body.get_operators_reader().unwrap();
        writeln!(out, "  LD IY,0").unwrap();
        writeln!(out, "  ADD IY,SP").unwrap();
        for op in operators {
            let op = op.unwrap();
            match op {
                Operator::LocalGet { local_index } => {
                    let d = params.len() as u32 * 4 - local_index * 4;
                    writeln!(out, "  ; local.get {}", local_index).unwrap();
                    writeln!(out, "  LD E,(IY+{})", d+2).unwrap();
                    writeln!(out, "  LD D,(IY+{})", d+3).unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                    writeln!(out, "  LD E,(IY+{})", d+0).unwrap();
                    writeln!(out, "  LD D,(IY+{})", d+1).unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                }
                Operator::LocalTee { local_index } => {
                    let d = params.len() as u32 * 4 - local_index * 4;
                    writeln!(out, "  ; local.tee {}", local_index).unwrap();
                    writeln!(out, "  LD IX,0").unwrap();
                    writeln!(out, "  ADD IX,SP").unwrap();
                    writeln!(out, "  LD A,(IX+0)").unwrap();
                    writeln!(out, "  LD (IY+{}),A", d+0).unwrap();
                    writeln!(out, "  LD A,(IX+1)").unwrap();
                    writeln!(out, "  LD (IY+{}),A", d+1).unwrap();
                    writeln!(out, "  LD A,(IX+2)").unwrap();
                    writeln!(out, "  LD (IY+{}),A", d+2).unwrap();
                    writeln!(out, "  LD A,(IX+3)").unwrap();
                    writeln!(out, "  LD (IY+{}),A", d+3).unwrap();
                },
                Operator::I32Const { value } => {
                    let lower = value as u16;
                    let upper = (value >> 16) as u16;
                    writeln!(out, "  ; i32.const").unwrap();
                    writeln!(out, "  LD DE,{lower}").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                    writeln!(out, "  LD DE,{upper}").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                }
                Operator::I32Store8 { memarg } => {
                    let offset = memarg.offset;
                    writeln!(out, "  ; i32.store8").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  LD BC,{offset}").unwrap();
                    writeln!(out, "  ADD IX,BC").unwrap();
                    writeln!(out, "  LD (IX+0),E").unwrap();
                }
                Operator::I32Load8U { memarg } => {
                    let offset = memarg.offset;
                    writeln!(out, "  ; i32.load8_u").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  LD BC,{offset}").unwrap();
                    writeln!(out, "  ADD IX,BC").unwrap();
                    writeln!(out, "  LD E,(IX+0)").unwrap();
                    writeln!(out, "  LD D,0").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                    writeln!(out, "  LD E,0").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                }
                Operator::I32Eqz => {
                    let zero = labeler.next();
                    let nonzero = labeler.next();
                    writeln!(out, "  ; i32.eqz").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  LD A,D").unwrap();
                    writeln!(out, "  OR E").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  OR D").unwrap();
                    writeln!(out, "  OR E").unwrap();
                    writeln!(out, "  JR Z,{zero}").unwrap();
                    writeln!(out, "  LD DE,0").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                    writeln!(out, "  JR {nonzero}").unwrap();
                    writeln!(out, "{zero}:").unwrap();
                    writeln!(out, "  LD DE,1").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                    writeln!(out, "{nonzero}:").unwrap();
                    writeln!(out, "  LD E,0").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                }
                Operator::I32Add => {
                    writeln!(out, "  ; i32.add").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP BC").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  POP HL").unwrap();

                    writeln!(out, "  AND A").unwrap();
                    writeln!(out, "  ADD HL,BC").unwrap();
                    writeln!(out, "  PUSH HL").unwrap();

                    writeln!(out, "  EX DE,HL").unwrap();
                    writeln!(out, "  PUSH IX").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  ADC HL,DE").unwrap();
                    writeln!(out, "  PUSH HL").unwrap();
                }
                Operator::I32And => {
                    writeln!(out, "  ; i32.and").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP BC").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  POP HL").unwrap();

                    writeln!(out, "  LD A,L").unwrap();
                    writeln!(out, "  AND C").unwrap();
                    writeln!(out, "  LD L,A").unwrap();
                    writeln!(out, "  LD A,H").unwrap();
                    writeln!(out, "  AND B").unwrap();
                    writeln!(out, "  LD H,A").unwrap();
                    writeln!(out, "  PUSH HL").unwrap();

                    writeln!(out, "  PUSH IX").unwrap();
                    writeln!(out, "  POP HL").unwrap();
                    writeln!(out, "  LD A,L").unwrap();
                    writeln!(out, "  AND E").unwrap();
                    writeln!(out, "  LD L,A").unwrap();
                    writeln!(out, "  LD A,H").unwrap();
                    writeln!(out, "  AND D").unwrap();
                    writeln!(out, "  LD H,A").unwrap();
                    writeln!(out, "  PUSH HL").unwrap();
                }
                Operator::I32GtU => {
                    let gt = labeler.next();
                    let after = labeler.next();

                    writeln!(out, "  ; i32.gt_u").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  POP HL").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP BC").unwrap();

                    writeln!(out, "  AND A").unwrap();
                    writeln!(out, "  SBC HL,BC").unwrap();
                    writeln!(out, "  LD B,H").unwrap();
                    writeln!(out, "  LD C,L").unwrap();
                    writeln!(out, "  PUSH IX").unwrap();
                    writeln!(out, "  POP HL").unwrap();
                    writeln!(out, "  SBC HL,DE").unwrap();

                    writeln!(out, "  JR C,{gt}").unwrap();
                    writeln!(out, "  LD HL,0").unwrap();
                    writeln!(out, "  PUSH HL").unwrap();
                    writeln!(out, "  JR {after}").unwrap();
                    writeln!(out, "{gt}:").unwrap();
                    writeln!(out, "  LD HL,1").unwrap();
                    writeln!(out, "  PUSH HL").unwrap();
                    writeln!(out, "{after}:").unwrap();
                    writeln!(out, "  LD HL,0").unwrap();
                    writeln!(out, "  PUSH HL").unwrap();
                },
                Operator::Select => {
                    let zero = labeler.next();
                    let after = labeler.next();
                    writeln!(out, "  ; select").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  LD A,D").unwrap();
                    writeln!(out, "  OR E").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  OR D").unwrap();
                    writeln!(out, "  OR E").unwrap();

                    writeln!(out, "  JR Z,{zero}").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP BC").unwrap();
                    writeln!(out, "  JR {after}").unwrap();
                    writeln!(out, "{zero}:").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  POP BC").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "  POP IX").unwrap();
                    writeln!(out, "{after}:").unwrap();
                    writeln!(out, "  PUSH BC").unwrap();
                    writeln!(out, "  PUSH DE").unwrap();
                },
                Operator::Br { relative_depth } => {
                    assert_eq!(relative_depth, 0);
                    writeln!(out, "  ; br").unwrap();
                    writeln!(out, "  JP {loop_label}").unwrap();
                }
                Operator::BrIf { relative_depth } => {
                    assert_eq!(relative_depth, 0);
                    writeln!(out, "  ; br_if").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  LD A,D").unwrap();
                    writeln!(out, "  OR E").unwrap();
                    writeln!(out, "  POP DE").unwrap();
                    writeln!(out, "  OR D").unwrap();
                    writeln!(out, "  OR E").unwrap();
                    writeln!(out, "  JP NZ,{loop_label}").unwrap();
                }
                Operator::Loop { blockty } => {
                    assert_eq!(blockty, BlockType::Empty);
                    writeln!(out, "{loop_label}:").unwrap();
                }
                Operator::Call { function_index } => {
                    let def = &self.functions[function_index as usize];
                    let num_locals = def.body.get_locals_reader().unwrap().get_count();
                    let params = def.func_type.params();
                    let results = def.func_type.results();
                    writeln!(out, "  ; call").unwrap();
                    writeln!(out, "  LD BC,0").unwrap();
                    for _ in 0..num_locals {
                        writeln!(out, "  PUSH BC").unwrap();
                        writeln!(out, "  PUSH BC").unwrap();
                    }
                    writeln!(out, "  PUSH IY").unwrap();
                    writeln!(out, "  CALL func_{}", function_index).unwrap();
                    if results.len() > 0 {
                        writeln!(out, "  POP DE").unwrap();
                        writeln!(out, "  POP BC").unwrap();
                    }
                    writeln!(out, "  POP IY").unwrap();
                    for _ in 0..(params.len() + num_locals as usize) {
                        writeln!(out, "  POP BC").unwrap();
                        writeln!(out, "  POP BC").unwrap();
                    }
                    if results.len() > 0 {
                        writeln!(out, "  PUSH BC").unwrap();
                        writeln!(out, "  PUSH DE").unwrap();
                    }
                }
                Operator::End => {}
                op => unimplemented!("operator {:?} not implemented", op),
            }
        }
        if has_result {
            writeln!(out, "  POP DE").unwrap();
            writeln!(out, "  POP BC").unwrap();
            writeln!(out, "  POP HL").unwrap();
            writeln!(out, "  PUSH BC").unwrap();
            writeln!(out, "  PUSH DE").unwrap();
            writeln!(out, "  PUSH HL").unwrap();
        }
        writeln!(out, "  RET").unwrap();
    }
}

struct Labeler {
    index: usize,
}

impl Labeler {
    fn new() -> Self {
        Self { index: 0 }
    }

    fn next(&mut self) -> Label {
        let index = self.index;
        self.index += 1;
        Label(index)
    }
}

struct Label(usize);
impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "label_{}", self.0)
    }
}
