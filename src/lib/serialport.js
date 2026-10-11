"use strict";

// Replacement for the native `serialport` Node module (v3 API subset used by
// the app), backed by the Rust serial commands in src-tauri/src/serial.rs.

import { invoke, Channel } from "@tauri-apps/api/core";
import { EventEmitter, inherits } from "./events.js";

function toError(error){
  return error instanceof Error ? error : new Error(error);
}

function callbackify(promise, callback){
  promise.then(function(result){
    if(callback)
      callback(null, result);
  }, function(error){
    if(callback)
      callback(toError(error));
  });
}

export function list(callback){
  invoke("serial_list").then(function(ports){
    callback(null, ports);
  }, function(error){
    callback(toError(error), []);
  });
}

export var parsers = {
  readline: function(delimiter){
    return { delimiter: delimiter || "\n" };
  }
};

export function SerialPort(path, options, openImmediately){
  EventEmitter.call(this);

  this.path = path;
  this.options = options || {};
  this.isOpen = false;

  if(openImmediately !== false)
    this.open();
}

inherits(SerialPort, EventEmitter);

SerialPort.prototype.open = function(callback){
  var that = this;
  var onEvent = new Channel();

  onEvent.onmessage = function(message){
    switch(message.event){
      case "data":
        that.emit("data", message.data);
        break;
      case "disconnected":
        that.isOpen = false;
        if(that.options.disconnectedCallback)
          that.options.disconnectedCallback(new Error(message.data));
        that.emit("disconnect", new Error(message.data));
        break;
    }
  };

  invoke("serial_open", {
    path: that.path,
    baudRate: that.options.baudrate || 9600,
    delimiter: (that.options.parser && that.options.parser.delimiter) || "\n",
    onEvent: onEvent
  }).then(function(){
    that.isOpen = true;
    that.emit("open");
    if(callback)
      callback(null);
  }, function(error){
    // Rust reports "Port is already open" with the same wording as serialport.
    if(callback)
      callback(toError(error));
  });
};

SerialPort.prototype.write = function(data, callback){
  callbackify(invoke("serial_write", { path: this.path, data: String(data) }), callback);
};

SerialPort.prototype.set = function(options, callback){
  callbackify(invoke("serial_set", { path: this.path, rts: !!options.rts, dtr: !!options.dtr }), callback);
};

SerialPort.prototype.close = function(callback){
  this.isOpen = false;
  callbackify(invoke("serial_close", { path: this.path }), callback);
};

export default {
  SerialPort: SerialPort,
  list: list,
  parsers: parsers
};
