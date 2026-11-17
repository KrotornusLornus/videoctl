local M = {}

function M.new(S, U, F, V)
  local D = {}

  function D.download()
    local paths = V.chosen_paths()
    if #paths == 0 or V.placeholder_selected(paths) then
      U.notify("No hay vídeo seleccionado.")
      return
    end
    if #U.lines(U.run_silent(S.bin .. " download-candidates " .. V.thumb_args(paths))) == 0 then
      U.notify("Nada que descargar: ya son locales o no tienen URL.")
      return
    end
    local qualities = {}
    local labels = {}
    for _, line in ipairs(U.lines(U.run_silent(S.bin .. " download-qualities"))) do
      local label, format = line:match("^(.-)\t(.*)$")
      if label and format then
        qualities[#qualities + 1] = { label = label, format = format }
        labels[#labels + 1] = label
      end
    end
    local selected_quality = F.menu(labels, "Calidad> ", "Elegir calidad")
    if not selected_quality then
      return
    end
    local format
    for _, item in ipairs(qualities) do
      if item.label == selected_quality then
        format = item.format
      end
    end
    if not format then
      return
    end
    local selected_dir = F.menu(
      U.lines(U.run_silent(S.bin .. " dirs")),
      "Carpeta> ",
      "Filtrar o escribir ruta"
    )
    if not selected_dir then
      return
    end
    local command = S.bin .. " download --format " .. U.q(format)
      .. " --dir " .. U.q(selected_dir) .. " " .. V.thumb_args(paths)
    local output = U.run_action(command)
    U.notify(output ~= "" and output or "Descarga iniciada.")
  end

  return D
end

return M
