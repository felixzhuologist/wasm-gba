const rust = import('./gba');
const wasm = import('./gba_bg');

const run = async () => {

const VM = await rust;
const { memory } = await wasm;

const armd = new cs.Capstone(cs.ARCH_ARM, cs.MODE_ARM);
const thumbd = new cs.Capstone(cs.ARCH_ARM, cs.MODE_THUMB);
let dis = armd;

let instruction_count = 0;
let bios_ptr = VM.get_bios();
let bg_palette_ptr = VM.get_bg_palette();
let sprite_palette_ptr = VM.get_bg_palette();
let rom;
let buf8 = new Uint8Array(memory.buffer);

const parseCpsr = (raw) => ({
    neg: (raw >> 31) & 1 == 1,
    zero: (raw >> 30) & 1 == 1,
    carry: (raw >> 29) & 1 == 1,
    overflow: (raw >> 28) & 1 == 1,
    irq: (raw >> 7) & 1 == 1,
    thumb: (raw >> 5) & 1 == 1,
    mode: getMode(raw & 0b11111),
})

const getMode = (raw) => {
    switch(raw) {
        case 0b10000:
            return "USR"
        case 0b10001:
            return "FIQ"
        case 0b10010:
            return "IRQ"
        case 0b10011:
            return "SVC"
        case 0b10111:
            return "ABT"
        case 0b11011:
            return "UND"
        case 0b11111:
            return "SYS"
        default:
            return raw.toString(2)
    }
}

const getFlag = (on, char) => on ? char : '-'

const updateSharedMem = () => {
    bios_ptr = VM.get_bios();
    bg_palette_ptr = VM.get_bg_palette();
    sprite_palette_ptr = VM.get_sprite_palette();
    buf8 = new Uint8Array(memory.buffer);
}

const addUploadListener = (id, callback) => {
    let input = document.getElementById(id);
    input.addEventListener("change", (event) => {
        let file = event.target.files[0];
        let reader = new FileReader();
        reader.onload = (event) => {
            let data = new Uint8Array(event.target.result);
            callback(data);
        };
        reader.readAsArrayBuffer(file);
    })
}

const addDebugListener = () => {
    const stepButton = document.getElementById('step');
    stepButton.addEventListener('click', event => step());
    const frameButton = document.getElementById('frame');
    frameButton.addEventListener('click', event => frame());
    const runButton = document.getElementById('bpsubmit')
    runButton.addEventListener("click", event => {
        let bp = parseInt(document.getElementById('bpinput').value, 16);
        run_until_break(bp);
    })
    const traceButton = document.getElementById('tracesubmit')
    traceButton.addEventListener("click", event => {
        let n = parseInt(document.getElementById('traceinput').value, 10);
        trace(n);
    })
}

const dumpState = () => {
    $("#count").text(instruction_count);
    $("#regs").empty();
    for (let i = 0; i < 16; i++) {
        $("#regs").append(
            // >>> "coerces" to an unsigned integer
            `<div class="col-md-3">R${i}: ${(VM.get_register(i) >>> 0).toString(16)}</div>`);
    }

    $("#pipeline").empty();
    let cpsr = parseCpsr(VM.get_cpsr());
    $("#pipeline").append(
        `<div class="col-md-4 col-md-offset-4">
            ${getFlag(cpsr.neg, 'N')}
            ${getFlag(cpsr.zero, 'Z')}
            ${getFlag(cpsr.carry, 'C')}
            ${getFlag(cpsr.overflow, 'V')}
            ${getFlag(cpsr.irq, 'I')}
            ${getFlag(cpsr.thumb, 'T')}
            ${cpsr.mode}
        </div>`);

    let pc = VM.get_register(15);
    let instr_size = dis === armd ? 4 : 2;
    let start = Math.max(0, pc - 2*instr_size);
    let end = pc + instr_size;
    try {
        let pipeline = dis.disasm(
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
    } catch(err) {
        console.error(err);
    }

    showPalette("#bg-palette", bg_palette_ptr);
    showPalette("#sprite-palette", sprite_palette_ptr);
}

const showPalette = (id, ptr) => {
    $(id).empty();
    for (let i = 0; i < 256; i++) {
        let blue = buf8[ptr + i*4];
        let green = buf8[ptr + i*4 + 1];
        let red = buf8[ptr + i*4 + 2];
        let color = `rgb(${red}, ${green}, ${blue})`;
        $(id).append(
            `<div class="palette-item" style="background-color: ${color}"></div>`)
    }
}

const step = () => {
    if (VM.step()) {
        pipelineFill();
    }
    instruction_count += 1;
    dis = parseCpsr(VM.get_cpsr()).thumb ? thumbd : armd;
    dumpState();
}

const frame = () => {
    VM.frame();
    dis = parseCpsr(VM.get_cpsr()).thumb ? thumbd : armd;
    dumpState();
}

const run_until_break = (breakpoint) => {
    let steps = 0;
    let started = false;
    while (!started || ((VM.get_register(15) - (dis == armd ? 8 : 4)) !== breakpoint)) {
        started = true;
        if (steps > 100000) { // don't hang indefinitely
            break;
        }
        if (VM.step()) {
            pipelineFill();
        }
        steps += 1;
    }
    instruction_count += steps;
    dis = parseCpsr(VM.get_cpsr()).thumb ? thumbd : armd;
    dumpState();
}

const trace = (n) => {
    for (let i = 0; i < n; i++) {
        if (VM.step()) {
            pipelineFill();
        }
    }
    instruction_count += n;
    dis = parseCpsr(VM.get_cpsr()).thumb ? thumbd : armd;
    dumpState();
}

const pipelineFill = () => {
    VM.step();
    VM.step();
}

const init = async () => {
    let bios = new Uint8Array(
        await fetch (`data/gba_bios.bin`).then(resp => resp.arrayBuffer()));
    VM.upload_bios(bios);
    let rom = new Uint8Array(
        await fetch (`data/sapphire.gba`).then(resp => resp.arrayBuffer()));
    VM.upload_rom(rom);
    updateSharedMem();
    dumpState();
    pipelineFill();
}

VM.set_panic_hook();
addUploadListener("bios", (data) => {
    VM.upload_bios(data);
    updateSharedMem();
    dumpState();
    // pipeline fill
    pipelineFill();
});
addUploadListener("rom", (data) => {
    VM.upload_rom(data);
    updateSharedMem();
    rom = data;
});
addDebugListener();
await init();
}

run();
