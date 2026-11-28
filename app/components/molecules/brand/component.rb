module Molecules
  module Brand
    class Component < ViewComponent::Base
      TONES = {
        light: "text-white",
        dark: "text-ink"
      }.freeze

      def initialize(tone: :dark)
        @tone = TONES.fetch(tone)
      end

      private
      attr_reader :tone

      def name_parts
        ApplicationHelper::APP_NAME.split("i", 2)
      end
    end
  end
end
