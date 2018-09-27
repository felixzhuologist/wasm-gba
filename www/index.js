const rust = import('./gba');
const wasm = import('./gba_bg');

const run = async () => {

const { GBA } = await rust;
const { memory } = await wasm;

const gba = GBA.new();
console.log(gba);

}

run();
