const http = require('http');

function post(method, params) {
    return new Promise((resolve) => {
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
            headers: { 'Content-Type': 'application/json', 'Content-Length': body.length }
        }, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => {
                try { resolve(JSON.parse(data)); } catch (e) { resolve({ error: e.message }); }
            });
        });
        req.on('error', (e) => resolve({ error: e.message }));
        req.write(body);
        req.end();
    });
}

async function testFormat(name, sceneRootValue) {
    console.log(`\n--- Testing Format: ${name} ---`);
    console.log("Payload:", JSON.stringify(sceneRootValue));
    
    const resp = await post("world.spawn_entity", {
        components: {
            "bevy_scene::components::SceneRoot": sceneRootValue,
            "bevy_transform::components::transform::Transform": {
                "translation": [0, 5, 0],
                "rotation": [0, 0, 0, 1],
                "scale": [1, 1, 1]
            }
        }
    });

    if (resp.error) {
        console.log("❌ FAILED:", resp.error.message || JSON.stringify(resp.error));
    } else if (resp.result) {
        console.log("✅ SUCCESS! Entity ID:", resp.result.entity);
        return true;
    } else {
        console.log("❓ UNKNOWN:", resp);
    }
    return false;
}

async function run() {
    const assetPath = "cube.glb#Scene0";

    // 1. Direct String (Some custom deserializers do this)
    // await testFormat("Direct String", assetPath); 

    // 2. Path Variant (If Bevy added it for Remote)
    await testFormat("Enum Variant: Path", { "Path": assetPath });

    // 3. The 'Handle' wrapper we tried (Strong/Uuid error)
    //    Wait, the error said: expected one of `Strong`, `Uuid`
    //    It didn't say `Path`. This strongly implies Remote Protocol DOES NOT SUPPORT PATH loading directly.
    //    But let's try assuming it might handle a raw map if it mimics the AssetLoader? No.
    
    // 4. Try 'Strong' with a fake ID? (Will fail deserialization if it expects UUID)
    // await testFormat("Enum Variant: Strong (Fake)", { "Strong": "12345" });

    // 5. Try the Reflection-based map (what we saw in similar crates)
    //    bevy_scene::components::SceneRoot(Handle<Scene>)
    //    Maybe: { "0": { ... } } ?
    await testFormat("Tuple Struct Index 0", { "0": { "Path": assetPath } });

    // 6. What if we just pass a simple map?
    await testFormat("Simple Map (type/path)", { "type": "Handle<...>", "path": assetPath });
}

run();
