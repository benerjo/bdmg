# business data model generator

## goal
The purpose of the code in this repository is to provide an easy way to generate the
necessary files to have objects that can be stored in a sqlite database.

## limitation
The purpose of this code is not to be fast/efficient, nor for the generated code to be fast/efficient. If it is, good thing! But no guarantee that after an upgrade, the code (or the generated code) will be faster or more efficient.

## general idea
The general idea is the following: each object has an unique integer id representing the object. 
The next value of the identifier can be found with the object Identifier. Moreover, each object has a version.
After each modification, the version of the object will be incremented.
When trying to set a new value to an attribute of the object, the value will be set only if the version of the object in memory is the same as the value in the database.

## usage
First, a user should define a json file containing the complete data model.
This json file can be read by bdmg2k in order to produce the corresponding rust code.
The generated code depends on the bdmg library.
This library define the necessary elements to have generic access on the datamodel.

## project
This project is developed under the AGPLv3.
For support (new features/bugs), please fill in an issue.

Note by by default, PR will be rejected, no matter the content.
Please, open an issue first. Otherwise, feel free to fork the project.
