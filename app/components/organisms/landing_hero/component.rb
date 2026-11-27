module Organisms
  module LandingHero
    class Component < ViewComponent::Base
      POINTS = %w[remember control trace].freeze

      PILLS = [
        { surface: "bg-graphite-50", text: "text-graphite-500", track: "bg-white", fill: "w-full", badge: "bg-graphite-900 text-sun-400" },
        { surface: "bg-graphite-50", text: "text-graphite-500", track: "bg-white", fill: "w-[62%]", badge: "bg-graphite-900 text-white" },
        { surface: "bg-graphite-900 text-white", text: "text-graphite-300", track: "bg-graphite-700", fill: "w-0", badge: "bg-sun-400 text-graphite-900" }
      ].freeze
    end
  end
end
