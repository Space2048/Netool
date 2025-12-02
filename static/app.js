document.addEventListener('DOMContentLoaded', () => {
    const pingBtn = document.getElementById('pingBtn');
    const speedBtn = document.getElementById('speedBtn');
    const portBtn = document.getElementById('portBtn');

    const pingResult = document.getElementById('pingResult');
    const speedResult = document.getElementById('speedResult');
    const portResult = document.getElementById('portResult');

    pingBtn.addEventListener('click', async () => {
        setLoading(pingBtn, pingResult);
        try {
            const res = await fetch('/api/ping');
            if (!res.ok) {
                const text = await res.text();
                throw new Error(text || res.statusText);
            }
            const data = await res.json();
            showResult(pingResult, `Pong! RTT: ${data.duration_ms}ms`);
        } catch (err) {
            showError(pingResult, err);
        } finally {
            resetLoading(pingBtn);
        }
    });

    speedBtn.addEventListener('click', async () => {
        const duration = parseInt(document.getElementById('speedDuration').value);
        setLoading(speedBtn, speedResult);
        try {
            const res = await fetch('/api/speed', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ duration })
            });
            if (!res.ok) {
                const text = await res.text();
                throw new Error(text || res.statusText);
            }
            const data = await res.json();
            showResult(speedResult,
                `Speed Test Finished:\n` +
                `Total Received: ${formatBytes(data.total_bytes)}\n` +
                `Duration: ${data.duration_secs.toFixed(2)}s\n` +
                `Speed: ${data.mbps.toFixed(2)} Mbps`
            );
        } catch (err) {
            showError(speedResult, err);
        } finally {
            resetLoading(speedBtn);
        }
    });

    portBtn.addEventListener('click', async () => {
        const range = document.getElementById('portRange').value;
        setLoading(portBtn, portResult);
        try {
            const res = await fetch('/api/ports', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ range })
            });
            if (!res.ok) {
                const text = await res.text();
                throw new Error(text || res.statusText);
            }
            const data = await res.json();
            showResult(portResult,
                `Port Test Complete:\n` +
                `Total Ports: ${data.total_ports}\n` +
                `Success: ${data.success_count}\n` +
                `Failed: ${data.fail_count}\n` +
                `Open Ports: ${data.open_ports.join(', ')}`
            );
        } catch (err) {
            showError(portResult, err);
        } finally {
            resetLoading(portBtn);
        }
    });

    function setLoading(btn, resultEl) {
        btn.disabled = true;
        btn.textContent = 'Running...';
        resultEl.style.display = 'none';
        resultEl.classList.remove('error');
    }

    function resetLoading(btn) {
        btn.disabled = false;
        btn.textContent = btn.id === 'pingBtn' ? 'Run Ping' :
            btn.id === 'speedBtn' ? 'Run Speed Test' : 'Run Port Test';
    }

    function showResult(el, text) {
        el.textContent = text;
        el.style.display = 'block';
    }

    function showError(el, err) {
        el.textContent = `Error: ${err.message || err}`;
        el.classList.add('error');
        el.style.display = 'block';
    }

    function formatBytes(bytes, decimals = 2) {
        if (!+bytes) return '0 Bytes';
        const k = 1024;
        const dm = decimals < 0 ? 0 : decimals;
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
    }
});
