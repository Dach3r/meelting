module ApplicationHelper
  CONFIG = { name: "Meelting" }

  def format_duration(seconds)
    minutes, secs = seconds.divmod(60)
    "#{minutes}m #{format("%02d", secs)}s"
  end
end
