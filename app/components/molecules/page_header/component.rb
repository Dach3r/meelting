module Molecules
  module PageHeader
    class Component < ViewComponent::Base
      renders_one :aside

      def initialize(title:, subtitle: nil, icon: nil, initial: nil, level: :h1, **html_options)
        @title = title
        @subtitle = subtitle
        @icon = icon
        @initial = initial
        @level = level
        @html_options = html_options
      end

      private
      attr_reader :title, :subtitle, :icon, :initial, :level, :html_options
    end
  end
end
