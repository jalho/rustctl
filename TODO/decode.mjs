import * as libfs from "node:fs";

function main() {
  const file_path = process.argv[2];
  const buf = libfs.readFileSync(file_path);
  const buf_hex = Buffer.from(buf.toString(), "hex");
  const utf8 = buf_hex.toString("utf8");
  console.log(utf8);
}

main();
