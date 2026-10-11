"use strict";

import fr from "./fr.json";
import en from "./en.json";

// Browser replacement for the `os-locale` module.
var osLocale = {
  sync: function(){
    return navigator.language || "en";
  }
};

var I18nClass = function I18nClass(){
  this.context = null;
}

I18nClass.prototype.initLanguage = function(){
  if(typeof localStorage!='undefined') {
    // Récupération de la valeur dans web storage
    var local = localStorage.getItem('local');
    // Vérification de la présence du compteur
    if(local!=null) {
      // Si oui, on convertit en nombre entier la chaîne de texte qui fut stockée
    } else {
      local = osLocale.sync();
    }
  } else {
    local = osLocale.sync();
  }

  this.setLanguage(local);
}

I18nClass.prototype.setLanguage = function(local){
  if(local.split(/[-_]/)[0] == "fr"){
    this.context = fr;
    this.currentLanguageID = "fr";
  }else{
    this.context = en;
    this.currentLanguageID = "en";
  }

  localStorage.setItem('local',local);
}

I18nClass.prototype.currentLanguage = function(){
  return this.context;
}

I18nClass.instance = null;

I18nClass.getInstance = function(){
    if(this.instance === null){
        this.instance = new I18nClass();
    }
    return this.instance;
}

export default I18nClass.getInstance();
