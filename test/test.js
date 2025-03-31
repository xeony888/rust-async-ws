const WebSocket = require("ws");


const SERVER_URL = "ws://localhost:8080";
const CLIENTS_COUNT = 1000;
const TEST_DURATION_SEC = 60;
const MESSAGES_PER_SEC = 10;
const letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
function generateRandomName() {
    return Array.from({ length: 20 }).map((_) => letters[Math.floor(Math.random() * letters.length)]).join("");
}
class StressTester {
    constructor() {
        this.stats = {
            connections: 0,
            messagesSentMove: 0,
            messagesReceivedMove: 0,
            messagesReceivedState: 0,
            messagesSentState: 0,
            errors: 0,
            startTime: 0,
            latencySumState: 0,
            latencySumMove: 0,
        };
    }

    async run() {
        console.log(`Starting stress test with ${CLIENTS_COUNT} clients for ${TEST_DURATION_SEC} seconds...`);
        this.stats.startTime = performance.now();

        // Create all clients
        const clients = [];
        for (let i = 0; i < CLIENTS_COUNT; i++) {
            clients.push(this.createClient(i));
        }

        // Run for specified duration
        await new Promise(resolve => setTimeout(resolve, TEST_DURATION_SEC * 1000));

        // Clean up
        clients.forEach(client => {
            if (client.readyState === WebSocket.OPEN) {
                client.close();
            }
        });

        this.printStats();
    }

    createClient(clientId) {
        const ws = new WebSocket(`${SERVER_URL}?name=${generateRandomName()}`);
        let interval;

        ws.on('open', () => {
            this.stats.connections++;
            console.log(`Client ${clientId} connected`);

            // Send periodic messages
            interval = setInterval(() => {
                const moveMsg = this.createMoveMessage();
                const start = performance.now();
                ws.send(moveMsg, (err) => {
                    if (err) {
                        this.stats.errors++;
                    } else {
                        this.stats.messagesSentMove++;
                        // Store latency for stats
                        ws.lastSentMove = start;
                    }
                });
                const stateMsg = this.createStateMessage();
                const start2 = performance.now();
                ws.send(stateMsg, (err) => {
                    if (err) {
                        this.stats.errors++;

                    } else {
                        this.stats.messagesSentState++;
                        ws.lastSentState = start2;
                    }
                })
            }, 1000 / MESSAGES_PER_SEC);
        });
        ws.on('message', (data) => {
            let type = 0;
            if (data) {
                type = this.getMessageType(data.buffer, data.byteOffset);
            }
            if (type === 2) {
                this.stats.messagesReceivedState++;
                if (ws.lastSentState) {
                    this.stats.latencySumState += performance.now() - ws.lastSentState
                }
            } else {
                // does not respond back on move message so this is useless;
                this.stats.messagesReceivedMove++;
                if (ws.lastSentMove) {
                    this.stats.latencySumMove += performance.now() - ws.lastSentMove;
                }
            }
        });

        ws.on('error', (err) => {
            this.stats.errors++;
            console.error(`Client ${clientId} error:`, err.message);
        });

        ws.on('close', () => {
            this.stats.connections--;
            if (interval) clearInterval(interval);
            // console.log(`Client ${clientId} disconnected`);
        });

        return ws;
    }
    getMessageType(buffer, offset) {
        if (buffer.byteLength === 0) {
            throw new Error("Buffer length 0");
        }
        return new DataView(buffer).getUint8(offset);
    }
    createMoveMessage() {
        // Create binary message matching your protocol
        const buffer = new ArrayBuffer(10); // 1 byte type + 16 bytes for two f64
        const view = new DataView(buffer);
        view.setUint8(0, 3); // MessageType::SoccerMove
        view.setFloat32(1, Math.random() * 2 - 1, true); // vx (-1 to 1)
        view.setFloat32(5, Math.random() * 2 - 1, true); // vy (-1 to 1)
        view.setUint8(9, Math.floor(Math.random() * 5));
        return buffer;
    }
    createStateMessage() {
        const buffer = new ArrayBuffer(1);
        const view = new DataView(buffer);
        view.setUint8(0, 2);
        return buffer;
    }

    printStats() {
        const durationSec = (performance.now() - this.stats.startTime) / 1000;
        const avgLatencyMove = this.stats.latencySumMove / Math.max(1, this.stats.messagesReceivedMove)
        const avgLatencyState = this.stats.latencySumState / Math.max(1, this.stats.messagesReceivedState);

        console.log('\n=== Stress Test Results ===');
        console.log(`Duration: ${durationSec.toFixed(2)} seconds`);
        console.log(`Peak connections: ${CLIENTS_COUNT}`);
        console.log(`State messages sent: ${this.stats.messagesSentState}`);
        console.log(`Move messages sent: ${this.stats.messagesSentMove}`);
        console.log(`State messages received: ${this.stats.messagesReceivedState}`);
        console.log(`Throughput: ${((this.stats.messagesReceivedState + this.stats.messagesReceivedMove) / durationSec).toFixed(2)} msg/sec`);
        console.log(`Avg latency Move: ${avgLatencyMove.toFixed(2)} ms`);
        console.log(`Avg latency State: ${avgLatencyState.toFixed(2)} ms`);
        console.log(`Errors: ${this.stats.errors}`);
    }
}

async function main() {
    const tester = new StressTester();
    await tester.run();

    tester.printStats();
}

main().then(() => console.log("DONE")).catch(console.error);
// Run the test
