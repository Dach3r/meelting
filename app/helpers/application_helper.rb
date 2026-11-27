module ApplicationHelper
  CONFIG = { name: "Meelting" }

  def format_cents(cents)
    format("$%d.%02d", cents / 100, cents % 100)
  end

  def format_duration(seconds)
    minutes, secs = seconds.divmod(60)
    "#{minutes}m #{format("%02d", secs)}s"
  end

  def format_clock(seconds)
    minutes, secs = seconds.divmod(60)
    format("%02d:%02d", minutes, secs)
  end
end
