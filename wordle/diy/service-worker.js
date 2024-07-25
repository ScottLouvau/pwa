const CACHE_VERSION = "v1";
const CACHE_PREFIX = "wordle-diy";

const CACHE_NAME = `${CACHE_PREFIX}-${CACHE_VERSION}`;
const CACHE_URLS = [
    "./",
    "./app.webmanifest",
    "./pwa/icon.svg",
];

async function install(event) {
    // Delete older caches for this app
    for (const name of await caches.keys()) {
        if (name.startsWith(CACHE_PREFIX) && name !== CACHE_NAME) {
            await caches.delete(name);
        }
    }

    // Activate the new service worker immediately
    event.waitUntil(self.skipWaiting());

    // Pre-cache specified resources in the current cache name
    const cache = await caches.open(CACHE_NAME);
    return cache.addAll(CACHE_URLS);
}

async function cacheFirst(request) {
    if (request.url.includes("localhost")) {
        return fetch(request);
    }

    const cachedResponse = await caches.match(request);
    if (cachedResponse) { 
        return cachedResponse; 
    }

    try {
        const networkResponse = await fetch(request);
        if (networkResponse.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, networkResponse.clone());
        }
        return networkResponse
    } catch (error) {
        return Response.error();
    }
}

async function deleteCaches() {
    // Delete all caches for this app
    for (const name of await caches.keys()) {
        await caches.delete(name);
    }
}

self.addEventListener("install", (event) => {
    event.waitUntil(install(event));
});

self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim()); 
});

self.addEventListener("fetch", (event) => {
    event.respondWith(cacheFirst(event.request));
});

self.addEventListener('message', async (event) => {
    if (event.data === 'deleteCaches') {
        console.log("Deleting Caches");
        await deleteCaches();
    } else if (event.data === 'getVersion') {
        event.source.postMessage(CACHE_VERSION);
    }
});