Feature: Step XX acceptance

  @auto @slice:step-xx
  Scenario: Core automated behavior
    Given fixture "step_xx_core"
    When I run "tonic check fixtures/step_xx_core"
    Then exit code is 0

  @agent-manual @slice:step-xx
  Scenario: Manual diagnostic quality check
    Given fixture "step_xx_error"
    When I run "tonic check fixtures/step_xx_error"
    Then diagnostic includes a specific next action

  @human-manual @slice:step-xx
  Scenario: Optional human readability check
    Given fixture "step_xx_error"
    When I inspect rendered diagnostic output
    Then the message is clear and non-ambiguous
