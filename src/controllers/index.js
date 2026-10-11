"use strict";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import pkg from "../../package.json";
import setApplicationMenu from "../lib/appMenu.js";
import DeviceManager from "../manager/devices.js";
import FirmwareController from "./firmware.js";
import PortSelectorController from "./portSelector.js";
import NavManager from "../manager/NavManager.js";
import ModalManager from "../manager/modalManager.js";
import ViewLoader from "./utils/ViewLoader.js";
import I18n from "../i18n/i18n.js";
I18n.initLanguage();

setApplicationMenu();

( function( $ ) {

  setView();

} )( window.jQuery );

function setView(){
  ViewLoader("container", function(content){
    $("body>#content").empty();
    $("body>#content").append(content);
    initPage();
  });
}

function initPage(){

  $(document).on('click', 'a[href^="http"], a[href^="mailto:"]', function(event) {
      event.preventDefault();
      openUrl(this.href);
  });

  NavManager.setContainer($("#pageContainer"));
  NavManager.setPage("home");
  $("#navHome").on("click", function(){
    NavManager.setPage("home");
  });
  $("#navBack").on("click", function(){
    NavManager.back();
  });

  DeviceManager.on("remove", function(device){
    if(device == DeviceManager.selectedDevice){
      DeviceManager.selectedDevice = null;

      if(NavManager.currentPage != "zoffset/3_printerConnection" && NavManager.currentPage != "home "){
        ModalManager.hideLoader();
        NavManager.setPage("home");
        ModalManager.alert(I18n.currentLanguage().disconnected_title, I18n.currentLanguage().disconnected_message);
      }
    }
  })

  var timeOutWelcome = setTimeout(function(){
    $("#welcome").fadeOut(1000)
  }, 4000);

  $("#welcome").on("click", function(){
    clearTimeout(timeOutWelcome);
    $("#welcome").fadeOut(100);
  });

  var $dropZone = $("html");

  $dropZone.on("dragover", function (e) {
    return false;
  });

  $dropZone.on("dragleave", function (e) {
    return false;
  });

  $dropZone.on("drop", function(e){
    if(NavManager.currentPage != "firmware/4_firmware")
      e.preventDefault();
  });

  /*var portSelectorController = new PortSelectorController()
  var firmwareController = new FirmwareController(portSelectorController);
  var diagnosticController = new DiagnosticController(portSelectorController);
  */

  $('.dropdown-button').dropdown();
  $("#languageFR").click(function(){
    $("#language img").attr("src", $(this).find("img").attr("src"));
    I18n.setLanguage("fr");
    setView();
  });

  $("#languageEN").click(function(){
    $("#language img").attr("src", $(this).find("img").attr("src"));
    I18n.setLanguage("en");
    setView();
  });

  $("#language img").attr("src", "/static/images/flags/"+I18n.currentLanguageID+".gif");

  $("#navbar a.close").click(function(){
    getCurrentWindow().close();
  });

  $('select').material_select();

  ModalManager.hideLoader();
  //ModalManager.setProgress(50);
  $('.tooltipped').tooltip({delay: 50});

  $("#version").text("V"+pkg.version);

  if(pkg.beta){
    $("body").addClass("beta");
    $("#version").append("<span class=\"beta\">b"+pkg.betaNumber+"</span>")
  }

}
