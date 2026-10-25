// Decode the Pasion server's public configuration without executing inline code.
(() => {
const block = document.getElementById('padmin-pasion-config');
if (block) {
  const match = /^\s*window\.APP_CONFIG\s*=\s*JSON\.parse\(("(?:[^"\\]|\\.)*")\);?\s*$/s.exec(block.textContent);
  if (!match) throw new Error('Invalid Pasion public configuration');
  const config = JSON.parse(JSON.parse(match[1]));
  if (!config || typeof config !== 'object' || Array.isArray(config)) throw new Error('Invalid Pasion configuration object');
  window.APP_CONFIG = config;
  block.remove();
}
})();
