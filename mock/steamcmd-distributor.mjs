/*
    $ node --version
    v20.14.0
*/

import * as http from "node:http";

function info(message, ...args) {
    console.log("[%s] - " + message, new Date().toISOString(), ...args);
}

function handle_request(i, o) {
    info("%s %s", i.method, i.url);

    switch (true) {
        case i.url.startsWith("/steamcmd.tgz"): {
            serve_dummy_steamcmd_tgz(i, o);
            return;
        }
        default: {
            o.statusCode = 404;
            o.end();
            return;
        }
    }

}

function serve_dummy_steamcmd_tgz(i, o) {
    o.statusCode = 200;
    const dummy_tgz = Buffer.from(
        "H4sIAAAAAAAAA+3RQQrCMBCF4Vl7ilxAyUicnCeYbirZtCno7bWWglBQF4VS+L/NW8wsHrw8lPI49rVJ5Vryqd6rrM6/WAhjarz4z5xoFA0WzevZgso7TJxfv8rS0NfUOSftLX39+3XfqTzu7+b9D1vXAQAAAAAAAAAAAAAAAAD86QkO9LboACgAAA==",
        "base64"
    );
    o.setHeader("content-length", dummy_tgz.byteLength);
    o.end(dummy_tgz);
}

(function main() {
    const server = http.createServer(handle_request);
    server.addListener("listening", () => info("Listening on %s", server.address().port));
    server.listen({ host: "127.0.0.1", port: process.argv[2] ?? 8080 });
})();
