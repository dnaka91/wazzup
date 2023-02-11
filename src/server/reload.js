var ws = new WebSocket('ws://localhost:8080/__WAZZUP__/reload');
ws.onopen = () => {
    console.log('connection opened');
};
ws.onerror = () => {
    console.log('failed connecting');
};
ws.onclose = () => {
    console.log('connection closed');
};
ws.onmessage = (ev) => {
    console.log(`got message "${ev.data}"`);
    if (ev.data === 'reload') {
        window.location.reload();
    }
};
