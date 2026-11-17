local M = {}

function M.new(S, U, F, V)
  local T = {}

  local function choose_method()
    local choice = F.menu(
      {
        "Automático",
        "ffmpeg (fotograma local)",
        "yt-dlp (thumbnail remoto)",
        "Cancelar",
      },
      "Miniatura> ",
      "Elegir método"
    )
    if not choice or choice == "Cancelar" then
      return nil
    end
    if choice == "ffmpeg (fotograma local)" then
      return "ffmpeg"
    end
    if choice == "yt-dlp (thumbnail remoto)" then
      return "ytdlp"
    end
    return "auto"
  end

  local function apply(command, message)
    local output = U.run_action(command)
    U.notify(output ~= "" and output or message)
    V.reload()
  end

  function T.repair()
    local paths = V.chosen_paths()
    local no_selection = #paths == 0 or V.placeholder_selected(paths)
    local action = F.menu(
      {
        "Regenerar selección/actual",
        "Regenerar todos los del filtro",
        "Completar faltantes del filtro",
        "Cancelar",
      },
      "Miniaturas> ",
      "Elegir acción"
    )
    if not action or action == "Cancelar" then
      return
    end
    if action == "Regenerar selección/actual" and no_selection then
      U.notify("No hay vídeo seleccionado.")
      return
    end
    local method = choose_method()
    if not method then
      return
    end
    if action == "Completar faltantes del filtro" then
      apply(
        S.bin .. " thumbs ensure --method " .. method .. " " .. V.filter_args(),
        "Miniaturas completadas."
      )
      return
    end
    local selector = V.filter_args()
    if action == "Regenerar selección/actual" then
      selector = V.thumb_args(paths)
    end
    apply(
      S.bin .. " thumbs regenerate --method " .. method .. " " .. selector,
      "Miniaturas regeneradas."
    )
  end

  return T
end

return M
