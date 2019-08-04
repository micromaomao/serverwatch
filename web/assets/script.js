for (let ele of document.querySelectorAll('.state_block')) {
	ele.addEventListener('click', evt => {
		let id = ele.href.match(/\#(e_\d+)/)
		if (id) {
			evt.preventDefault();
			id = id[1]
			let element_to_show = document.getElementById(id)
			if (element_to_show) {
				show_detail(element_to_show)
			}
		}
	})
}

function hide_all() {
	for (let existing of document.querySelectorAll('.detail')) {
		Object.assign(existing.style, {
			height: '0',
			opacity: '0',
			display: 'block'
		})
	}
}

hide_all()

function show_detail(element_to_show) {
	hide_all()
	element_to_show.style.display = 'block'
	element_to_show.style.height = 'auto'
	fix_timestamps()
	let computed_height = parseInt(window.getComputedStyle(element_to_show).height)
	element_to_show.style.height = '0'
	element_to_show.style.opacity = '0'
	requestAnimationFrame(() => {
		element_to_show.style.height = computed_height + 'px' // transistion
		element_to_show.style.opacity = '1'
	})
}

function fix_timestamps() {
	for (let t of document.querySelectorAll('.timestamp')) {
		let time = new Date(parseInt(t.dataset.time))
		t.textContent = `${time.toLocaleTimeString()} (${Math.floor((Date.now() - time) / 1000)} secs ago)`
	}
}

setInterval(fix_timestamps, 1000)
