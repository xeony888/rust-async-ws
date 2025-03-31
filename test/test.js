const WebSocket = require("ws");


const SERVER_URL = "ws://localhost:8080";
const CLIENTS_COUNT = 100;
const TEST_DURATION_SEC = 30;
const MESSAGES_PER_SEC = 10;

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
        const ws = new WebSocket(SERVER_URL);
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
            if (data.data) {
                type = this.getMessageType(data.data);
            }
            if (type === 2) {
                this.stats.messagesReceivedState++;
                if (ws.lastSentState) {
                    this.stats.latencySumState += performance.now() - ws.lastSentState
                }
            } else {
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
    getMessageType(buffer) {
        if (buffer.byteLength === 0) {
            throw new Error("Buffer length 0");
        }
        const view = new DataView(buffer);
        let offset = 0;
        const gameType = view.getUint8(offset);
        return gameType;
    }
    createMoveMessage() {
        // Create binary message matching your protocol
        const buffer = new ArrayBuffer(17); // 1 byte type + 16 bytes for two f64
        const view = new DataView(buffer);
        view.setUint8(0, 3); // MessageType::SoccerMove
        view.setFloat64(1, Math.random() * 2 - 1, true); // vx (-1 to 1)
        view.setFloat64(9, Math.random() * 2 - 1, true); // vy (-1 to 1)
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
