"use strict";

import i18n from "../../i18n/i18n.js";

// Templates are bundled at build time instead of being read from disk with `fs`.
var views = import.meta.glob("../../views/**/*.html", { query: "?raw", import: "default", eager: true });

export default function loadFile(file, callback){
  //console.log("loadFile",file);
  var that = this;
  var data = views["../../views/"+file+".html"];
  //console.log("file readed");
  var template = Handlebars.compile(data);
  //console.log("file compiled");
  var context = i18n.currentLanguage();
  var html = template(context);
  //console.log("file i18n done");
  callback(html);
};
