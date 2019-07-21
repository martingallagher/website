const
  staticDir = "assets/static/",
  fs = require('fs'),
  csso = require('csso');

fs.readdir(staticDir, function (err, items) {
  items.forEach(file => {
    if (!file.endsWith('.css')) {
      return;
    }

    const
      filename = staticDir + file,
      data = fs.readFileSync(filename, 'utf8'),
      optimized = csso.minify(data).css;

    fs.writeFileSync(filename, optimized);

    const diff = Math.round(((data.length - optimized.length) / data.length) * 10000) / 100;

    console.log(`${file} ${data.length} > ${optimized.length} (${diff}%)`);
  });
});
