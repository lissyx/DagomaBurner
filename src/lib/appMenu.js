"use strict";

// Port of the Electron `Menu.setApplicationMenu(template)` the windows used to set.

import { invoke } from "@tauri-apps/api/core";
import { Menu } from "@tauri-apps/api/menu";
import platform from "./platform.js";

var template = [
  {
    text: 'DagomApp',
    items: [
      { item: { About: { name: 'DagomApp' } }, text: 'DagomApp' },
      { item: 'Separator' },
      { item: 'Hide', text: 'Hide DagomApp' },
      { item: 'HideOthers', text: 'Hide Others' },
      { item: 'ShowAll', text: 'Show All' },
      { item: 'Separator' },
      { item: 'Quit', text: 'Quit' },
    ]
  },
  {
    text: 'View',
    items: [
      {
        text: 'Reload',
        accelerator: 'Command+R',
        action: function(){
          window.location.reload();
        }
      },
      {
        text: 'Toggle Developer Tools',
        accelerator: 'Alt+Command+I',
        action: function(){
          invoke("toggle_devtools");
        }
      }
    ]
  },
  {
    text: 'Edit',
    items: [
      { item: 'Undo', text: 'Undo' },
      { item: 'Redo', text: 'Redo' },
      { item: 'Separator' },
      { item: 'Cut', text: 'Cut' },
      { item: 'Copy', text: 'Copy' },
      { item: 'Paste', text: 'Paste' },
      { item: 'SelectAll', text: 'Select All' },
    ]
  },
  {
    text: 'Window',
    items: [
      { item: 'Minimize', text: 'Minimize' },
      { item: 'CloseWindow', text: 'Close' },
      { item: 'Separator' },
      { item: 'BringAllToFront', text: 'Bring All to Front' },
    ]
  }
];

export default function setApplicationMenu(){
  // The template only uses macOS menu roles. Electron hid the menu bar on the
  // other platforms (autoHideMenuBar), so Tauri doesn't attach one there.
  if(platform != "darwin")
    return Promise.resolve();

  return Menu.new({ items: template }).then(function(menu){
    return menu.setAsAppMenu();
  });
}
