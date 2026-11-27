module Organisms
  module LandingStats
    class Component < ViewComponent::Base
      STATS = %w[sandbox secrets providers license].freeze
      TINTS = %w[bg-brand-50 bg-sun-100 bg-coral-100 bg-sky-100].freeze
    end
  end
end
