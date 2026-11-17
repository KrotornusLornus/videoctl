local M = {}

function M.new(S, U, V, A, P, C, D, G, H, T)
  local K = {}

  local function toggle_music()
    S.music = not S.music
    V.update_title(swayimg.imagelist.size)
    U.notify(S.music and "Modo música activado." or "Modo música desactivado.")
  end

  local function toggle_text()
    S.help_visible = false
    swayimg.text.visible = not swayimg.text.visible
  end

  function K.bind()
    swayimg.gallery.on_key({ "h" }, function() swayimg.gallery.select("left") end)
    swayimg.gallery.on_key({ "l" }, function() swayimg.gallery.select("right") end)
    swayimg.gallery.on_key({ "k" }, function() swayimg.gallery.select("up") end)
    swayimg.gallery.on_key({ "j" }, function() swayimg.gallery.select("down") end)

    swayimg.gallery.on_key({ "space", "Insert" }, A.toggle_mark)
    swayimg.gallery.on_key({ "Shift+space" }, A.clear_marks)
    swayimg.gallery.on_key({ "Return", "KP_Enter" }, function() P.play(false) end)
    swayimg.gallery.on_key({ "Shift+Return", "Shift+KP_Enter" }, function() P.play(true) end)
    swayimg.gallery.on_key({ "Shift+m" }, toggle_music)
    swayimg.gallery.on_key({ "Shift+s" }, C.scan)
    swayimg.gallery.on_key({ "Shift+i" }, C.import_urls)
    swayimg.gallery.on_key({ "slash", "Shift+slash" }, C.pick_category)
    swayimg.gallery.on_key({ "Ctrl+f" }, C.pick_dir)
    swayimg.gallery.on_key({ "a" }, C.assign_category)
    swayimg.gallery.on_key({ "Shift+c" }, C.manage_categories)
    swayimg.gallery.on_key({ "w" }, D.download)
    swayimg.gallery.on_key({ "i" }, G.info)
    swayimg.gallery.on_key({ "Shift+question", "question" }, H.toggle)
    swayimg.gallery.on_key({ "y" }, function() G.copy(false) end)
    swayimg.gallery.on_key({ "Shift+y" }, function() G.copy(true) end)
    swayimg.gallery.on_key({ "d", "Delete" }, A.remove_from_view)
    swayimg.gallery.on_key({ "Shift+d" }, G.delete_video)
    swayimg.gallery.on_key({ "Shift+r" }, T.repair)
    swayimg.gallery.on_key({ "r" }, V.reload)
    swayimg.gallery.on_key({ "t" }, toggle_text)
    swayimg.gallery.on_key({ "f" }, function() swayimg.fullscreen = not swayimg.fullscreen end)
    swayimg.gallery.on_key({ "equal", "plus", "Shift+plus", "KP_Add", "Shift+KP_Add" }, function()
      swayimg.gallery.thumb_size = swayimg.gallery.thumb_size + 10
    end)
    swayimg.gallery.on_key({ "minus", "Shift+minus", "KP_Subtract" }, function()
      swayimg.gallery.thumb_size = swayimg.gallery.thumb_size - 10
    end)
    swayimg.gallery.on_key({ "s" }, function() U.notify("Modo presentación desactivado.") end)
    swayimg.gallery.on_key({ "Escape" }, function() swayimg.exit() end)
  end

  return K
end

return M
