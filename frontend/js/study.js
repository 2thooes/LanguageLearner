function renderCourses(courses) {
    const list = document.getElementById('coursesList');
    list.innerHTML = ''; // Очищаем список перед отрисовкой

    if (!courses || courses.length === 0) {
        const emptyItem = document.createElement('li');
        emptyItem.textContent = 'No courses yet';
        list.appendChild(emptyItem);
        return;
    }

    for (const [index, course] of courses.entries()) {
        const li = document.createElement('li');
        const link = document.createElement('a');

        // Если ID совпадает с индексом массива (0, 1, 2...):
        link.href = `/course/${index}`;
        // Если ID в базе начинаются с 1 (1, 2, 3...), используйте: `/course/${index + 1}`

        link.textContent = course;

        li.appendChild(link);
        list.appendChild(li);
    }
}

async function getName() {
    const res = await fetch('/api/me', {
        credentials: 'same-origin'
    });

    if (res.ok) {
        const data = await res.json();
        document.getElementById('username').textContent = data.login;
        console.log(data.login);
    } else {
        window.location.href = "/login";
        return;
    }

    const coursesRes = await fetch('/api/mycourses', {
        credentials: 'same-origin'
    });

    if (coursesRes.ok) {
        const data = await coursesRes.json();
        console.log("::" + JSON.stringify(data));
        renderCourses(data.courses);
    } else {
        window.location.href = "/login";
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