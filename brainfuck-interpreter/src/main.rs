fn build_bracket_map(program: &str) -> Vec<usize> {
    let bytes = program.as_bytes();
    let len = bytes.len();
    let mut map = vec![0usize; len];
    let mut stack: Vec<usize> = Vec::new();

    for i in 0..len {
        match bytes[i] {
            b'[' => stack.push(i),
            b']' => {
                let open = stack.pop().expect("Unmatched ]");
                map[open] = i;
                map[i] = open;
            }
            _ => {}
        }
    }
    if !stack.is_empty() {
        panic!("Unmatched [");
    }
    map
}

fn main() {
    let program = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
    let mut tape = [0u8; 30000];
    let mut dp: usize = 0;
    let mut pc: usize = 0;

    let bracket_map = build_bracket_map(program);
    let bytes = program.as_bytes();

    while pc < bytes.len() {
        match bytes[pc] {
            b'>' => dp += 1,
            b'<' => dp -= 1,
            b'+' => tape[dp] = tape[dp].wrapping_add(1),
            b'-' => tape[dp] = tape[dp].wrapping_sub(1),
            b'.' => print!("{}", tape[dp] as char),
            b',' => { /* input not needed for Hello World */ }
            b'[' => {
                if tape[dp] == 0 {
                    pc = bracket_map[pc];
                }
            }
            b']' => {
                if tape[dp] != 0 {
                    pc = bracket_map[pc];
                }
            }
            _ => {}
        }
        pc += 1;
    }

    println!();
}
