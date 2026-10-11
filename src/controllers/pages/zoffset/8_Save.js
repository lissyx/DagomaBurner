"use strict";

import ViewLoader from "../../utils/ViewLoader.js";
import NavManager from "../../../manager/NavManager.js";
import ModalManager from "../../../manager/modalManager.js";
import GCodeSender from "../../utils/GCodeSender.js";
import GCodeParser from "../../utils/GCodeParser.js";
import GCodePrinter from "../../utils/GCodePrinter.js";
import I18n from "../../../i18n/i18n.js";

var ZoffsetSaveClass = function ZoffsetSaveClass(){
  this.content = null;
}

ZoffsetSaveClass.prototype.load = function (callback) {
  var that = this;
  if(that.content)
    return callback();

  ViewLoader("zoffset/8_Save", function(content){
    that.content = $(content);
    that.initView();
    if(callback){
      callback();
    }
  });
};

ZoffsetSaveClass.prototype.initView = function () {
  var that = this;
  that.content.find("#next").on("click", function(){
    NavManager.setPage("home");
  });
};

ZoffsetSaveClass.prototype.show = function () {
  var that = this;
  ModalManager.showLoader(I18n.currentLanguage().z_offset_printing_saving);
  that.printer  = new GCodePrinter();
  that.content.hide();
  GCodeSender.send([
    "M114",
    "G90",
    "G90"],
    false,
    function(response){
      console.log("response", response);


      var currentZ, currentZOffset;
      var regex = /X:(\d+.\d+) Y:(\d+.\d+) Z:(-?\d+.\d+)/.exec(response);
      console.log("regex", regex);
      if(regex && regex.length>=3)
        currentZ = -parseFloat(regex[3]);
      /*var regex = /M851 Z(-?\d+.\d+)/.exec(response);
      if(regex && regex.length>1)
        currentZOffset = parseFloat(regex[1]);
      console.log(regex);*/
      var newZOffset = -10 - 0.26 - currentZ;
      newZOffset = newZOffset.toFixed(2);

      console.log("response M114 M503", "currentZ", currentZ, "newZOffset", newZOffset);

      GCodeSender.send([
        "M851 Z"+newZOffset,
        "M500"
      ],
      false,
      function(){
        that.printer.finishPrint(function(){
          ModalManager.hideLoader();
          that.content.show();
          that.content.find("#zOffsetValue").text(newZOffset);
        });
      })
    });
};

ZoffsetSaveClass.prototype.dispose = function () {
};

export default ZoffsetSaveClass;
