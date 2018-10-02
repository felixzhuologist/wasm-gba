const rust = import('./gba');
const wasm = import('./gba_bg');

const run = async () => {

const VM = await rust;
const { memory } = await wasm;

const armd = new cs.Capstone(cs.ARCH_ARM, cs.MODE_ARM);
const thumd = new cs.Capstone(cs.ARCH_ARM, cs.MODE_THUMB);

let reg_ptr = VM.get_registers() / 4;
let bios_ptr = VM.get_bios();
let buf32 = new Uint32Array(memory.buffer);
let buf8 = new Uint8Array(memory.buffer);

const parse_cpsr = (raw) => ({
    neg: (raw >> 31) & 1 == 1,
    zero: (raw >> 30) & 1 == 1,
    carry: (raw >> 29) & 1 == 1,
    overflow: (raw >> 28) & 1 == 1,
    irq: (raw >> 7) & 1 == 1,
    thumb: (raw >> 5) & 1 == 1,
    mode:
        (raw & 0b11111) === 0b10000
        ? "USR"
        : (raw & 0b11111) === 0b10011
        ? "SVC"
        : "unknown",
})

const get_flag = (on, char) => on ? char : '-'

const update_shared_mem = () => {
    reg_ptr = VM.get_registers() / 4;
    bios_ptr = VM.get_bios();
    buf32 = new Uint32Array(memory.buffer);
    buf8 = new Uint8Array(memory.buffer);
}

const addBiosListener = () => {
    let input = document.getElementById("bios");
    input.addEventListener("change", (event) => {
        let bios = event.target.files[0];
        let reader = new FileReader();
        reader.onload = (event) => {
            let data = new Uint8Array(event.target.result);
            VM.upload_rom(data);
            update_shared_mem();
            dumpState();
            // pipeline fill
            step();
            step();
        };
        reader.readAsArrayBuffer(bios);
    })
}

const addDebugListener = () => {
    const stepButton = document.getElementById('step');
    stepButton.addEventListener("click", event => step());
}

const dumpState = () => {
    $("#regs").empty();
    for (let i = 0; i < 16; i++) {
        $("#regs").append(
            `<div class="col-md-3">R${i}: ${buf32[reg_ptr + i].toString(16)}</div>`);
    }

    $("#pipeline").empty();
    let cpsr = parse_cpsr(VM.get_cpsr());
    $("#pipeline").append(
        `<div class="col-md-4 col-md-offset-4">
            ${get_flag(cpsr.neg, 'N')}
            ${get_flag(cpsr.zero, 'Z')}
            ${get_flag(cpsr.carry, 'C')}
            ${get_flag(cpsr.overflow, 'V')}
            ${get_flag(cpsr.irq, 'I')}
            ${get_flag(cpsr.thumb, 'T')}
            ${cpsr.mode}
        </div>`);

    let pc = buf32[reg_ptr + 15];
    let start = Math.max(0, pc - 8);
    let end = pc + 4;
    let pipeline = armd.disasm(
        buf8.slice(bios_ptr + start, bios_ptr + end),
        start);
    pipeline.forEach((instr) => {
        $("#pipeline").append(
            `<div class="col-md-8 col-md-offset-4">
                ${instr.address.toString(16)}:
                    (${instr.bytes.map(
                        (x) => x.toString(16).padStart(2, '0')).join(' ')})
                    ${instr.mnemonic} ${instr.op_str}
            </div>`
        )
    })
}

const step = () => {
    VM.step();
    dumpState();
}

addBiosListener();
addDebugListener();
}

run();
