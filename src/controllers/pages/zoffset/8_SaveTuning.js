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
  that.content.hide();

  that.content.find("#zOffsetValue").text(window.zoffset.toFixed(2));

  GCodeSender.send([
    "M851 Z"+window.zoffset.toFixed(2),
    "M500"
  ],
  false,
  function(){
    ModalManager.hideLoader();
    that.content.show();
  })
};

ZoffsetSaveClass.prototype.dispose = function () {
};

export default ZoffsetSaveClass;
