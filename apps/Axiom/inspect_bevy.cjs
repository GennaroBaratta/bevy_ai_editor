const http = require('http');

function post(method, params) {
    return new Promise((resolve, reject) => {
        const body = JSON.stringify({
            jsonrpc: "2.0",
            method: method,
            id: 1,
            params: params
        });

        const req = http.request({
            hostname: '127.0.0.1',
            port: 15721,
            path: '/',
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Content-Length': body.length
            }
        }, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => {
                try {
                    resolve(JSON.parse(data));
                } catch (e) {
                    reject(e);
                }
            });
        });

        req.on('error', reject);
        req.write(body);
        req.end();
    });
}

async function inspect() {
    try {
        console.log("Fetching all entities with Transform...");
        // 1. Get all entities that have a Transform (almost all do)
        // Corrected format for Bevy 0.18 BRP world.query: { "data": { "components": [...] } }
        const listResp = await post("world.query", {
            data: {
                components: ["bevy_transform::components::transform::Transform"]
            }
        });

        if (!listResp.result) {
            console.error("No entities found or error:", listResp);
            return;
        }

        console.log(`Found ${listResp.result.length} entities. Scanning for SceneRoot...`);

        // 2. Iterate and check components for EACH entity
        for (const item of listResp.result) {
            const entityId = item.entity;
            console.log(`Checking Entity ${entityId}...`);
            
            const detailResp = await post("world.get_components", {
                entity: entityId,
                components: [
                    "bevy_scene::components::SceneRoot",
                    "bevy_transform::components::transform::Transform"
                ]
            });

            if (detailResp.error) {
                // Ignore errors (likely component not present on this entity)
                // console.error(`Error getting components for ${entityId}:`, detailResp.error);
                continue;
            }

            if (detailResp.result) {
                const components = detailResp.result;
                // console.log(`Components:`, Object.keys(components));

                // Check if we got SceneRoot
                if (components["bevy_scene::components::SceneRoot"]) {
                    console.log("\n!!! FOUND SCENE ROOT !!!");
                    console.log("Entity ID:", entityId);
                    console.log("JSON Structure:", JSON.stringify(components["bevy_scene::components::SceneRoot"], null, 2));
                    return; // Found it!
                }
            }
        }
        console.log("No SceneRoot found in any entity.");

    } catch (e) {
        console.error("Error:", e);
    }
}

inspect();
