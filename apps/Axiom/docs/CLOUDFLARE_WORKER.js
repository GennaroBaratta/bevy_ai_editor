export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    
    // Target: Google Generative AI (Gemini)
    // If your client speaks OpenAI protocol (v1/chat/completions),
    // ensure the target endpoint supports it or add translation logic here.
    // As of 2025/2026, many use this for IP masking or caching.
    const TARGET_HOST = "https://generativelanguage.googleapis.com";

    // Construct the destination URL
    const destinationUrl = TARGET_HOST + url.pathname + url.search;

    // Create a new request with the original method, headers, and body
    const newRequest = new Request(destinationUrl, {
      method: request.method,
      headers: request.headers,
      body: request.body,
      redirect: "follow"
    });

    // You can inject the API key here if you want to hide it from the client
    // if (env.GEMINI_API_KEY) {
    //   newRequest.headers.set("x-goog-api-key", env.GEMINI_API_KEY);
    // }

    try {
      const response = await fetch(newRequest);
      return response;
    } catch (e) {
      return new Response(JSON.stringify({ error: e.message }), {
        status: 500,
        headers: { "content-type": "application/json" }
      });
    }
  }
};
