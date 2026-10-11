"use strict";

import { invoke } from "@tauri-apps/api/core";

// `file` is the File picked in the page (browsers don't expose its path), and
// `type` the bootloader name ("melzi" or "mega"). Flashing used to go through
// avrgirl-arduino; it is now done by src-tauri/src/flasher, which also holds
// the board definitions.
export default function(device, file, type, callback){
  if(device == null)
    return callback(false);

  console.log("burn to ", device);

  var timeOut3DPrinterSearch;
  var success = true;
  device.isBuilding = true;
  device.pause();

  console.log("ready to flash");
  setTimeout(function(){
    console.log("flash");

    function flash(hex, done){
      invoke("flash_firmware", {
        port: device.portName,
        hex: hex,
        board: type
      }).then(function(){
        done(null);
      }, done);
    }

    function printerFound(){
      clearTimeout(timeOut3DPrinterSearch);
      device.removeListener("printerFound", printerFound);
      callback(success);
    }

    function resetAndWaitPort(success){
      device.isBuilding = false;
      setTimeout(function(){device.open()}, 1000);

      timeOut3DPrinterSearch = setTimeout(function(){
        //TODO: treat when no printer is discovered
        //no3DPrinterFound();
        printerFound();
      }, 30000);
      device.on("printerFound", printerFound);
    }

    file.text().then(function(hex){
      flash(hex, function (error) {
        if (error) {
          console.log("retrying after error", error);
          //callback(false);

          //device.resetPort();
          // Retry immediatly
          setTimeout( function() {
            flash(hex, function (error) {

              if (error) {
                console.log("ok, giving up with error", error);
                success = false;
              } else {
                console.log('done.');
                success = true;
              }

              resetAndWaitPort(true/*, result.message*/);
            });
          }, 5000);

        } else {
          console.log('done.');
          success = true;
          resetAndWaitPort(true/*, result.message*/);
        }
      });
    }, function(error){
      console.log("can't read firmware file", error);
      success = false;
      resetAndWaitPort(true);
    });

  },
  5000);
};
