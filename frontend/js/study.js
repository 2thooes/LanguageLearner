let realCourses = [];

function setUrlState(paramsObj, replace = false) {
    const url = new URL(window.location.href);
    url.search = '';
    for (const [key, value] of Object.entries(paramsObj)) {
        url.searchParams.set(key, String(value));
    }
    if (replace) {
        window.history.replaceState(null, '', url);
    } else {
        window.history.pushState(null, '', url);
    }
}

function renderSidebar(activeType, activeIndex = null) {
    const list = document.getElementById('coursesList');
    list.innerHTML = '';

    realCourses.forEach((title, index) => {
        const li = document.createElement('li');
        const link = document.createElement('a');
        link.href = `?course=${index}`;
        link.textContent = title;

        if (activeType === 'course' && activeIndex === index) {
            link.classList.add('active');
        }

        link.addEventListener('click', (e) => {
            e.preventDefault();
            setUrlState({ course: index });
            openView('course', index);
        });

        li.appendChild(link);
        list.appendChild(li);
    });

    // Пункт "Add new" всегда внизу меню
    const addLi = document.createElement('li');
    const addLink = document.createElement('a');
    addLink.href = '?view=add';
    addLink.textContent = 'Add new';

    if (activeType === 'add') {
        addLink.classList.add('active');
    }

    addLink.addEventListener('click', (e) => {
        e.preventDefault();
        setUrlState({ view: 'add' });
        openView('add');
    });

    addLi.appendChild(addLink);
    list.appendChild(addLi);
}

function renderCoursesByLang(lang) {
        coursesContainer.innerHTML = '';
        if (!lang) return;

        const matching = data.filter((c) => c.user_language_code === lang);
        if (matching.length === 0) {
            coursesContainer.textContent = 'No courses for this language';
            return;
        }

        const ul = document.createElement('ul');
        for (const course of matching) {
            const li = document.createElement('li');
            li.textContent = `${course.title} (${course.user_language_code} -> ${course.target_language_code})`;
            ul.appendChild(li);
        }
        coursesContainer.appendChild(ul);
    }

async function openView(type, index = null) {
    const pane = document.getElementById('contentPane');
    renderSidebar(type, index);

    if (type === 'add') {
        pane.innerHTML = `
            <h2>Add new course</h2>
            <div id="languageList"></div>
        `;
        const availableCourses = await fetch(`/api/availableCourses`);//дописать тут
        
        const data = await availableCourses.json(); //Getting all possible courses
        const languageSet = new Set(); //Selcting only user languages
        for (const course of data) {
            if (course.user_language_code) {
                languageSet.add(course.user_language_code);
            }
        }

        const languageList = document.getElementById("languageList");

        const currentParams = new URLSearchParams(window.location.search);//тут читаем поисковую строку
        let selectedLang = currentParams.get('lang');

        for (const lang of languageSet) {
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.textContent = lang;

            btn.addEventListener('click', () => {
                selectedLang = lang;
                // Сохраняем view=add и добавляем/обновляем параметр lang в адресной строке
                setUrlState({ view: 'add', lang });
                renderCoursesByLang(selectedLang);
            });

            languageList.appendChild(btn);
        }

        console.log(languageSet);
        return;
    }

    const courseTitle = realCourses[index];
    pane.innerHTML = `
        <h2>${courseTitle}</h2>
        <p id="courseBody">Loading...</p>
    `;

    // Запрос к БД за содержимым конкретного курса
    const res = await fetch(`/api/courses/${index}`, { credentials: 'same-origin' });
    const bodyEl = document.getElementById('courseBody');
    if (!bodyEl) return;

    if (res.ok) {
        const courseData = await res.json();
        bodyEl.textContent = JSON.stringify(courseData);
    } else {
        bodyEl.textContent = 'Failed to load course data';
    }
}

function syncWithUrl() {
    const params = new URLSearchParams(window.location.search);
    const view = params.get('view');
    const courseParam = params.get('course');

    if (view === 'add') {
        openView('add');
        return;
    }

    if (courseParam !== null) {
        const idx = Number(courseParam);
        if (Number.isInteger(idx) && idx >= 0 && idx < realCourses.length) {
            openView('course', idx);
            return;
        }
    }

    // Если параметры не заданы или некорректны — выбираем старт по умолчанию
    if (realCourses.length > 0) {
        setUrlState({ course: 0 }, true);
        openView('course', 0);
    } else {
        setUrlState({ view: 'add' }, true);
        openView('add');
    }
}

async function initStudyPage() {
    const meRes = await fetch('/api/me', { credentials: 'same-origin' });
    if (!meRes.ok) {
        window.location.href = '/login';
        return;
    }
    const meData = await meRes.json();
    document.getElementById('username').textContent = meData.login;

    const coursesRes = await fetch('/api/mycourses', { credentials: 'same-origin' });
    if (!coursesRes.ok) {
        window.location.href = '/login';
        return;
    }
    const coursesData = await coursesRes.json();

    // На случай если бэкенд пока всё ещё кладёт "Add new" последним элементом в массив
    const rawList = Array.isArray(coursesData.courses) ? coursesData.courses : [];
    realCourses = rawList.filter((item) => item !== 'Add new');// change

    syncWithUrl();
}

window.addEventListener('popstate', () => {
    if (realCourses.length > 0 || window.location.search) {
        syncWithUrl();
    }
});

initStudyPage();