let wasm = null;
let ws = null;

export default function() {
    let xTouchDown = null;
    const container = document.getElementById('container');
    const crypto = document.getElementById('crypto');
    const status = document.getElementById('status');
    const public_key = document.getElementById('public_key');
    const challenge = document.getElementById('challenge');
    const response = document.getElementById('response');

    let pendingChallenge = null;

    function createSlider(key, label) {
        const slider = document.createElement('div');
        slider.className = 'slider';

        let execute = false;

        function updateSlider(xDiff) {
            const sliderWidth = slider.offsetWidth;
            const totalWidth = container.offsetWidth;
            const percent = 1 / (totalWidth - sliderWidth) * xDiff;

            if (percent > 0.9) {
                xDiff = totalWidth - sliderWidth;
                slider.textContent = 'OK';
                execute = true;
            } else {
                slider.textContent = '>>';
                execute = false;
            }

            slider.style.marginLeft = xDiff + 'px';
        }

        slider.addEventListener('touchstart', function(event) {
            xTouchDown = event.touches[0].clientX;
        }, false);
        slider.addEventListener('touchmove', function(event) {
            if (!xTouchDown) {
                return;
            }

            const xTouchCurrent = event.touches[0].clientX;
            const xDiff = xTouchCurrent - xTouchDown;
            if (xDiff <= 0) {
                return;
            }

            updateSlider(xDiff);
        }, false);
        slider.addEventListener('touchend', function(_event) {
            if (execute) {
                if (ws) {
                    const msg = JSON.stringify({
                        "type": "fetch",
                        "door": key,
                    });
                    pendingChallenge = key;
                    console.log('send cmd to websocket:', msg);
                    ws.send(msg);
                }
            }

            slider.style.marginLeft = null;
            xTouchDown = null;
            updateSlider(0);
        }, false);

        updateSlider(0);

        const h1 = document.createElement('h1');
        h1.textContent = label;
        container.appendChild(h1);
        container.appendChild(slider);
    }

    function connect() {
        const websocketUrl = (document.location.protocol === 'https:' ? 'wss://' : 'ws://') + document.location.host + document.location.pathname;
        ws = new WebSocket(websocketUrl);

        ws.onmessage = event => {
            container.classList.remove('offline');
            const data = JSON.parse(event.data);

            if (data.type === 'challenge') {
                if (pendingChallenge === null) {
                    return;
                }

                // put challenge to html
                challenge.value = data.challenge;

                // invoke web assembly
                if (!wasm.solve_challenge()) {
                    console.log("Web assembly failed to decrypt");
                    return;
                }

                // read respnse
                const code = response.value;

                let msg = JSON.stringify({
                    type: "solve",
                    door: pendingChallenge,
                    code: code,
                });
                pendingChallenge = null;
                console.log('send cmd to websocket:', msg);
                ws.send(msg);
            } else if (data.type === 'config') {
                while (container.firstChild) {
                    container.removeChild(container.lastChild);
                }

                public_key.value = data.public_key;
                data.doors.forEach(data => {
                    createSlider(data['id'], data['label']);
                });
            }
        };

        ws.onclose = event => {
            console.log('WebSocket closed with code: ' + event.code + ', reason: ' + event.reason);
            reconnect();
        };

        ws.onerror = error => {
            console.log('WebSocket error:', error);
            reconnect();
        };
    }

    const reconnectInterval = 2000; // Time in milliseconds between reconnection attempts
    let reconnectTimeout = null;
    function reconnect() {
        container.classList.add('offline');
        console.log('reconnecting...');
        clearInterval(reconnectTimeout);
        reconnectTimeout = setTimeout(connect, reconnectInterval);
    }

    connect();

    status.textContent = 'crypto: STARTING';
    return function(WASM) {
        wasm = WASM;

        function validate_key() {
            if (wasm.validate_key()) {
                status.textContent = 'crypto: OK';
            } else {
                status.textContent = 'crypto: MISSING KEY';
            }
        }

        status.addEventListener('click', function(event) {
            event.preventDefault();
            crypto.hidden = !crypto.hidden;
            container.hidden = !container.hidden;
        });

        challenge.addEventListener('change', function() {
            wasm.solve_challenge();
        });

        window.addEventListener('hashchange', validate_key);
        validate_key();
    };
}
