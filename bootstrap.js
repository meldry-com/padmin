import init from './pkg/padmin.js';
init().catch(error => { document.getElementById('app').textContent = 'Unable to start Padmin: ' + error; });
