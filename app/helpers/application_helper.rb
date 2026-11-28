module ApplicationHelper
  CONFIG = { name: "Meelting" }
  SIDEBAR_COOKIE = "sidebar_expanded".freeze

  def sidebar_expanded?
    cookies[SIDEBAR_COOKIE] == "1"
  end

  def format_duration(seconds)
    minutes, secs = seconds.divmod(60)
    "#{minutes}m #{format("%02d", secs)}s"
  end
end
