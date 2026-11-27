module Molecules
  module Brand
    class Component < ViewComponent::Base
      TONES = {
        light: { text: "text-ink", dot: "bg-brand-500" },
        deep: { text: "text-white", dot: "bg-sun-400" }
      }.freeze

      def initialize(tone: :light)
        @tone = TONES.fetch(tone)
      end

      private
      attr_reader :tone
    end
  end
end
