const rust = import('./gba');
const wasm = import('./gba_bg');

const run = async () => {

const VM = await rust;
const { memory } = await wasm;

const addBiosListener = () => {
    let input = document.getElementById("bios");
    input.addEventListener("change", (event) => {
        let bios = event.target.files[0];
        let reader = new FileReader();
        reader.onload = (event) => {
            let data = new Uint8Array(event.target.result);
            console.log(data.length);
            VM.upload_rom(data);
        };
        reader.readAsArrayBuffer(bios);
    })
}

// const reg_ptr = VM.get_registers();
// const buf = new Uint32Array(memory.buffer);
// const val = reg_ptr / 4;
// console.log(val, buf.length);
// console.log(buf[val]);
// console.log(buf[val + 15]);

addBiosListener();

}

run();
