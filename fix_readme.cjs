
const fs = require('fs');
const files = fs.readdirSync('.').filter(f => f.startsWith('README') && f.endsWith('.md'));
const langLinks = '\n**Languages**: [English](README.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [???](README.ja.md) | [???](README.ko.md) | [Ti?ng Vi?t](README.vi.md) | [????](README.zh-hans.md) | [????](README.zh-hant.md)\n';

files.forEach(file => {
    let content = fs.readFileSync(file, 'utf8');
    if (content.includes('[English](README.md) | [Français]')) return;
    content = content.replace(/(#\s+.*Fish[^\n]*\n)/i, $1);
    fs.writeFileSync(file, content);
});
console.log('Updated ' + files.length + ' README files.');

