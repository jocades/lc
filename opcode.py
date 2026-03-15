with open("opcodes.txt") as f:
    opcodes = ""
    as_str = """
pub fn as_str(opcode: u8) -> &'static str {
    match opcode {\n"""
    count = 1
    for line in f.read().splitlines():
        opcode = line.strip()

        if not opcode:
            opcodes += "\n"
            as_str += "\n"
            continue

        opcodes += f"pub const {opcode}: u8 = {count};\n"
        as_str += f'        {opcode} => "{opcode}",\n'
        count += 1
    as_str += f"        0 | {count}..=u8::MAX => unreachable!(),\n"
    as_str += "    }\n}\n"

    print(opcodes)
    print(as_str)
