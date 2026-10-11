"use strict";

import ViewLoader from "../../utils/ViewLoader.js";
import NavManager from "../../../manager/NavManager.js";
import ModalManager from "../../../manager/modalManager.js";
import CodeBuilder from "../../../manager/codeBuilder.js";
import GCodeSender from "../../utils/GCodeSender.js";
import GCodeParser from "../../utils/GCodeParser.js";
import GCodePrinter from "../../utils/GCodePrinter.js";
import I18n from "../../../i18n/i18n.js";
import DeviceManager from "../../../manager/devices.js";

var PIDRunClass = function PIDRunClass(){
  this.content = null;
}

PIDRunClass.prototype.load = function (callback) {

  var that = this;
  if(that.content)
    return callback();

  ViewLoader("reset/4_reset", function(content){
    that.content = $(content);
    that.initView();
    if(callback){
      callback();
    }
  });
};

PIDRunClass.prototype.initView = function () {
  var that = this;
  that.content.find("#next").on("click", function(){
    NavManager.setPage("home");
  });
};

PIDRunClass.prototype.show = function () {
  var that = this;

  that.device = DeviceManager.selectedDevice;

  that.content.hide();
  ModalManager.showLoader();
  GCodeSender.send([
    "M502",
    "M500"],
    false,
    function(result){
      console.log("result M502/M500", result);
      that.content.show();
      ModalManager.hideLoader();
    }
  );
};

PIDRunClass.prototype.dispose = function () {

};

export default PIDRunClass;
