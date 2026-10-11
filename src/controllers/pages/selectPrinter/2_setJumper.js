"use strict";

import ViewLoader from "../../utils/ViewLoader.js";
import NavManager from "../../../manager/NavManager.js";
import DeviceManager from "../../../manager/devices.js";


var ZoffsetSetJumperMelziClass = function ZoffsetSetJumperMelziClass(){
  this.content = null;
}

ZoffsetSetJumperMelziClass.prototype.load = function (callback) {
  var that = this;
  if(that.content)
    return callback();

  ViewLoader("selectPrinter/2_setJumper", function(content){
    that.content = $(content);
    that.initView();
    if(callback){
      callback();
    }
  });
};

ZoffsetSetJumperMelziClass.prototype.initView = function () {

  var that = this;
  that.content.find("#next").on("click", function(){
    NavManager.setPage("selectPrinter/3_printerConnection")
  });

  $("#navBack").show();
}

ZoffsetSetJumperMelziClass.prototype.show = function () {

};

ZoffsetSetJumperMelziClass.prototype.dispose = function () {
};

export default ZoffsetSetJumperMelziClass;
