document.getElementById('loginForm').addEventListener('submit', async (e) => {
    e.preventDefault();
            
    const login = document.getElementById('login').value;
    const password = document.getElementById('password').value;
    const messageDiv = document.getElementById('message');

    messageDiv.textContent = 'Sending...';

    try {
        const response = await fetch('/api/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ login, password })
        });

        const data = await response.json();

        if (response.ok) {
            messageDiv.textContent = 'Success';
            setTimeout(() => {
                window.location.href = '/study';
            }, 1000);
        } else {
            messageDiv.textContent = data.message || 'Login error';
        }
    } catch (err) {
        messageDiv.textContent = 'Error in server connection';
    }
});