import React from 'react';

declare global {
    interface Window {
        Vue: any;
        pytketCircuitDisplays: { [key: string]: any };
    }
}

function loadScript(src: string): Promise<void> {
    return new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = src;
        script.onload = () => resolve();
        script.onerror = () => reject(new Error(`Failed to load script: ${src}`));
        document.head.appendChild(script);
    });
}

function loadStylesheet(href: string): Promise<void> {
    return new Promise((resolve, reject) => {
        const link = document.createElement('link');
        link.rel = 'stylesheet';
        link.href = href;
        link.onload = () => resolve();
        link.onerror = () => reject(new Error(`Failed to load stylesheet: ${href}`));
        document.head.appendChild(link);
    });
}

function renderCircuit(element: HTMLElement, circuitJson: string) {

    const uid = "whatever";

    // Set up global variables expected by main.js
    (window as any).circuitRendererUid = uid;
    (window as any).displayOptions = {};

    // Create a container for the Vue app
    const container = document.createElement('div');
    container.id = `circuit-display-vue-container-${uid}`;
    container.innerHTML = `
        <div style="display: none">
            <div id="circuit-json-to-display-${uid}">${circuitJson}</div>
        </div>
        <circuit-display-container
            :circuit-element-str="'#circuit-json-to-display-${uid}'"
            :init-render-options="initRenderOptions"
        ></circuit-display-container>
    `;
    element.appendChild(container);

    // Create the Vue app
    const { createApp } = window.Vue;
    const circuitDisplayContainer = (window as any)["pytket-circuit-renderer"].default;
    const app = createApp({
        delimiters: ['[[#', '#]]'],
        components: { circuitDisplayContainer },
        data() {
            return {
                initRenderOptions: (window as any).displayOptions,
            };
        }
    });
    app.config.unwrapInjectedRef = true;
    app.mount(`#circuit-display-vue-container-${uid}`);

    if (typeof window.pytketCircuitDisplays === "undefined") {
        window.pytketCircuitDisplays = {};
    }
    window.pytketCircuitDisplays[uid] = app;
}

async function loadResources() {
    // Ensure Vue is loaded
    if (!window.Vue) {
        await loadScript('https://unpkg.com/vue@3/dist/vue.global.js');
    }
    await loadScript('https://unpkg.com/pytket-circuit-renderer@0.9/dist/pytket-circuit-renderer.umd.js')

    await loadStylesheet('https://unpkg.com/pytket-circuit-renderer@0.9/dist/pytket-circuit-renderer.css');
}

// Hook for React components
export default function useCircuitViewer(containerRef: React.RefObject<HTMLDivElement>, circuitJson: string) {
    React.useEffect(() => {
        const loadAndRender = async () => {
            await loadResources();
            if (containerRef.current) {
                renderCircuit(containerRef.current, circuitJson);
            }
        };

        loadAndRender();
    }, [containerRef, circuitJson]);
}