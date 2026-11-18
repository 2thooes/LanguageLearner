async function getName(){
                const res = await fetch('/api/me');
                if (res.ok){
                    const data = await res.json();
                    document.getElementById('username').textContent = data.login;
                    console.log(data.login);
                }
                else{
                    window.location.href="/login";
                }
        }
        getName();
        document.getElementById('logoutBtn').addEventListener('click', async () => {
            try {
                const response = await fetch('/api/logout', {
                    method: 'POST'
                });
                if (response.ok) {
                    // Сервер стер куку, перенаправляем на страницу входа
                    window.location.href = '/login';
                } else {
                    alert('Error');
                }
            } catch (err) {
                console.error('Error', err);
            }
        });