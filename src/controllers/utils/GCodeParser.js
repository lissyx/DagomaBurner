// G-code files are bundled at build time instead of being streamed from disk with `fs`.
var files = import.meta.glob("../../*.g", { query: "?raw", import: "default", eager: true });

export default function(sName, fCallback) {
  var result = [],
      aLine,
      lines = files["../../"+sName].split(/\r?\n/);

  // Like readline, don't report the empty "line" after the final newline.
  if(lines[lines.length-1] == "")
    lines.pop();

  lines.forEach(function (line) {
    aLine = line.trim();
    if(line[0]==";")
      aLine = null;
    if(aLine != null)
      result.push(aLine);
  });

  fCallback(result);
}
