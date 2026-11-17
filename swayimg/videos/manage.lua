local M = {}

function M.new(S, U, F, V)
  local G = {}

  function G.delete_video()
    local paths = V.chosen_paths()
    if #paths == 0 or V.placeholder_selected(paths) then
      U.notify("No hay vídeo seleccionado.")
      return
    end
    local action = F.menu(
      { "Cancelar", "Solo borrar metadata", "Borrar fichero + metadata" },
      "Borrar> ",
      "Elegir acción"
    )
    if not action or action == "Cancelar" then
      return
    end
    local command = S.bin .. " delete"
    if action == "Borrar fichero + metadata" then
      command = command .. " --files"
    end
    local output = U.run_action(command .. " " .. V.thumb_args(paths))
    U.notify(output ~= "" and output or "Operación terminada.")
    V.reload()
  end

  function G.info()
    local path = V.current_path()
    if not path then
      return
    end
    local output = U.run_action(S.bin .. " info --thumb " .. U.q(path))
    U.notify(output ~= "" and output or "Sin información.")
  end

  function G.copy(both)
    local paths = V.chosen_paths()
    if #paths == 0 or V.placeholder_selected(paths) then
      U.notify("No hay vídeo seleccionado.")
      return
    end
    local flag = both and " --both" or ""
    local output = U.run_action(S.bin .. " copy" .. flag .. " " .. V.thumb_args(paths))
    U.notify(output ~= "" and output or "Copiado.")
  end

  return G
end

return M
