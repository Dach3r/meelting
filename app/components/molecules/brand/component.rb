module Molecules
  module Brand
    class Component < ViewComponent::Base
      TONES = {
        light: { text: "text-ink", mark: "text-brand-600" },
        deep: { text: "text-white", mark: "text-sun-400" }
      }.freeze

      def initialize(tone: :light)
        @tone = TONES.fetch(tone)
      end

      private
      attr_reader :tone
    end
  end
end
