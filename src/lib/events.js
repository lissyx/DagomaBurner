"use strict";

// Browser replacement for Node's `events.EventEmitter` and `util.inherits`,
// which the Electron renderer got from Node integration.

export function EventEmitter(){
  this._events = {};
}

EventEmitter.prototype.on = function(name, listener){
  if(!this._events)
    this._events = {};
  (this._events[name] = this._events[name] || []).push(listener);
  return this;
};

EventEmitter.prototype.addListener = EventEmitter.prototype.on;

EventEmitter.prototype.once = function(name, listener){
  var that = this;
  function onceListener(){
    that.removeListener(name, onceListener);
    listener.apply(that, arguments);
  }
  onceListener.listener = listener;
  return this.on(name, onceListener);
};

EventEmitter.prototype.removeListener = function(name, listener){
  var listeners = this._events && this._events[name];
  if(!listeners)
    return this;

  // Like Node, remove only the most recently added matching listener.
  for(var i = listeners.length - 1; i >= 0; i--){
    if(listeners[i] === listener || listeners[i].listener === listener){
      listeners.splice(i, 1);
      break;
    }
  }
  return this;
};

EventEmitter.prototype.off = EventEmitter.prototype.removeListener;

EventEmitter.prototype.removeAllListeners = function(name){
  if(!this._events)
    return this;
  if(name === undefined)
    this._events = {};
  else
    delete this._events[name];
  return this;
};

EventEmitter.prototype.listeners = function(name){
  return ((this._events && this._events[name]) || []).slice();
};

EventEmitter.prototype.emit = function(name){
  var listeners = this._events && this._events[name];
  if(!listeners || listeners.length == 0){
    if(name == "error")
      throw arguments[1];
    return false;
  }

  var args = Array.prototype.slice.call(arguments, 1);
  // Iterate over a copy so listeners can remove themselves while being called.
  listeners.slice().forEach(function(listener){
    listener.apply(this, args);
  }, this);
  return true;
};

export function inherits(ctor, superCtor){
  Object.setPrototypeOf(ctor.prototype, superCtor.prototype);
}
